package com.example.topk;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.configuration.Configuration;
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.functions.ProcessFunction;
import org.apache.flink.util.Collector;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import redis.clients.jedis.Jedis;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.*;

/**
 * Top-K Flink job.
 *
 * Reads JSON events {user_id, item_id, ts} from Kafka, maintains:
 *   - one global (all-time) Count-Min Sketch
 *   - one per-current-hour Count-Min Sketch (rotated on hour boundary)
 *   - a bounded candidate set per window (Space-Saving style, CAPACITY=5000)
 * and periodically flushes both sketches + candidate heaps to Redis under keys
 * that the Rust query API reads (see topk_common::keys).
 *
 * Byte layout is fixed and MUST match topk_common::CountMinSketch:
 *   width(u32 LE) | depth(u32 LE) | width*depth * u32 LE counters
 *
 * Parallelism is 1 by design: CMS is additive but we don't want to partition the
 * item universe across subtasks for this demo (would force a merge step). 10M
 * items at Zipf load fit comfortably in a single 75KB sketch on one TM.
 */
public class TopKJob {

    private static final Logger LOG = LoggerFactory.getLogger(TopKJob.class);

    // ===== CMS parameters — MUST match topk_common =====
    static final int CMS_WIDTH = 2719;
    static final int CMS_DEPTH = 7;
    static final long MERSENNE_P = (1L << 61) - 1L;
    static final long[] HASH_A = {
        0x9E3779B97F4A7C15L, 0xBF58476D1CE4E5B9L, 0x94D049BB133111EBL,
        0xD6E8FEB86659FD93L, 0xA24BAED4963EE407L, 0x85EBCA6B2A6D3F27L,
        0xC2B2AE3D27D4EB4FL,
    };
    static final long[] HASH_B = {
        0x165667B19E3779F9L, 0x3C6EF372FE94F82BL, 0xA54FF53A5F1D36F1L,
        0x510E527FADE682D1L, 0x9B05688C2B3E6C1FL, 0x1F83D9ABFB41BD6BL,
        0x5BE0CD19137E2179L,
    };
    static final int CANDIDATE_CAPACITY = 5000;
    static final long FLUSH_EVERY_MS = 5_000L;
    /// Per-minute sketches kept for 7 days; all_time has no TTL.
    static final int MINUTE_TTL_SEC = 7 * 24 * 60 * 60;

    public static void main(String[] args) throws Exception {
        final String brokers = env("KAFKA_BROKERS", "kafka:9092");
        final String topic = env("KAFKA_TOPIC", "events");
        final String redisHost = env("REDIS_HOST", "redis");
        final int redisPort = Integer.parseInt(env("REDIS_PORT", "6379"));

        StreamExecutionEnvironment se = StreamExecutionEnvironment.getExecutionEnvironment(new Configuration());
        se.setParallelism(1);

        KafkaSource<String> source = KafkaSource.<String>builder()
            .setBootstrapServers(brokers)
            .setTopics(topic)
            .setGroupId("topk-flink")
            .setStartingOffsets(OffsetsInitializer.latest())
            .setValueOnlyDeserializer(new org.apache.flink.api.common.serialization.SimpleStringSchema())
            .build();

        DataStream<String> raw = se.fromSource(source, WatermarkStrategy.noWatermarks(), "kafka-events");
        raw.process(new TopKProcess(redisHost, redisPort)).name("topk-aggregator");

        se.execute("topk-flink-job");
    }

    private static String env(String k, String d) {
        String v = System.getenv(k);
        return v == null || v.isEmpty() ? d : v;
    }

    // ===== Core operator =====
    static class TopKProcess extends ProcessFunction<String, Void> {
        private final String redisHost;
        private final int redisPort;

        private transient ObjectMapper mapper;
        private transient Jedis jedis;
        private transient int[] allTime;             // flat row-major counters
        private transient int[] minute;              // per-current-minute sketch
        private transient long currentMinute;
        private transient long lastFlushMs;

        // Bounded item-count maps for candidate heaps.
        private transient LinkedHashMap<String, Long> allTimeCandidates;
        private transient LinkedHashMap<String, Long> minuteCandidates;

        TopKProcess(String redisHost, int redisPort) {
            this.redisHost = redisHost;
            this.redisPort = redisPort;
        }

        @Override
        public void open(Configuration parameters) {
            this.mapper = new ObjectMapper();
            this.jedis = new Jedis(redisHost, redisPort);
            this.allTime = new int[CMS_WIDTH * CMS_DEPTH];
            this.minute = new int[CMS_WIDTH * CMS_DEPTH];
            this.currentMinute = epochMinute(System.currentTimeMillis() / 1000L);
            this.lastFlushMs = System.currentTimeMillis();
            this.allTimeCandidates = new LinkedHashMap<>(CANDIDATE_CAPACITY * 2);
            this.minuteCandidates = new LinkedHashMap<>(CANDIDATE_CAPACITY * 2);
        }

        @Override
        public void close() {
            if (jedis != null) {
                try { flush(); } catch (Exception e) { LOG.warn("final flush failed", e); }
                jedis.close();
            }
        }

        @Override
        public void processElement(String json, Context ctx, Collector<Void> out) {
            String itemId;
            long ts;
            try {
                JsonNode n = mapper.readTree(json);
                itemId = n.get("item_id").asText();
                ts = n.has("ts") ? n.get("ts").asLong() : System.currentTimeMillis() / 1000L;
            } catch (Exception e) {
                return; // drop malformed
            }

            long m = epochMinute(ts);
            if (m != currentMinute) {
                // Minute rolled over — flush old minute sketch under its epoch_min key, then reset.
                flushMinute(currentMinute);
                Arrays.fill(minute, 0);
                minuteCandidates.clear();
                currentMinute = m;
            }

            long itemX = itemHash64(itemId.getBytes(StandardCharsets.UTF_8));
            long estAll = bumpAndEstimate(allTime, itemX);
            long estMin = bumpAndEstimate(minute, itemX);
            updateCandidates(allTimeCandidates, itemId, estAll);
            updateCandidates(minuteCandidates, itemId, estMin);

            long now = System.currentTimeMillis();
            if (now - lastFlushMs >= FLUSH_EVERY_MS) {
                flush();
                lastFlushMs = now;
            }
        }

        private long bumpAndEstimate(int[] sketch, long x64) {
            int min = Integer.MAX_VALUE;
            for (int r = 0; r < CMS_DEPTH; r++) {
                int col = rowHash(r, x64);
                int idx = r * CMS_WIDTH + col;
                int v = sketch[idx];
                if (v != Integer.MAX_VALUE) {
                    v += 1;
                    sketch[idx] = v;
                }
                if (v < min) min = v;
            }
            return (long) min & 0xFFFFFFFFL;
        }

        private void updateCandidates(LinkedHashMap<String, Long> map, String item, long est) {
            map.put(item, est);
            if (map.size() > CANDIDATE_CAPACITY) {
                // Evict lowest-estimate entry (Space-Saving lite).
                String victim = null;
                long victimEst = Long.MAX_VALUE;
                for (Map.Entry<String, Long> e : map.entrySet()) {
                    if (e.getValue() < victimEst) {
                        victim = e.getKey();
                        victimEst = e.getValue();
                    }
                }
                if (victim != null) map.remove(victim);
            }
        }

        private void flush() {
            flushAllTime();
            flushMinute(currentMinute);
        }

        private void flushAllTime() {
            byte[] cms = serialize(allTime);
            jedis.set("topk:cms:all_time".getBytes(StandardCharsets.UTF_8), cms);
            jedis.set("topk:heap:all_time", heapJson(allTimeCandidates));
        }

        private void flushMinute(long minuteEpoch) {
            byte[] cms = serialize(minute);
            String cmsKey = "topk:cms:min:" + minuteEpoch;
            String heapKey = "topk:heap:min:" + minuteEpoch;
            jedis.setex(cmsKey.getBytes(StandardCharsets.UTF_8), MINUTE_TTL_SEC, cms);
            jedis.setex(heapKey, MINUTE_TTL_SEC, heapJson(minuteCandidates));
        }

        private String heapJson(LinkedHashMap<String, Long> map) {
            // Sort desc by est, keep top CANDIDATE_CAPACITY, emit JSON matching HeavyHitter { item, est }.
            List<Map.Entry<String, Long>> list = new ArrayList<>(map.entrySet());
            list.sort((a, b) -> Long.compare(b.getValue(), a.getValue()));
            StringBuilder sb = new StringBuilder();
            sb.append('[');
            for (int i = 0; i < list.size(); i++) {
                if (i > 0) sb.append(',');
                Map.Entry<String, Long> e = list.get(i);
                sb.append("{\"item\":");
                appendJsonString(sb, e.getKey());
                sb.append(",\"est\":").append(e.getValue()).append('}');
            }
            sb.append(']');
            return sb.toString();
        }

        private static void appendJsonString(StringBuilder sb, String s) {
            sb.append('"');
            for (int i = 0; i < s.length(); i++) {
                char c = s.charAt(i);
                if (c == '"' || c == '\\') sb.append('\\').append(c);
                else if (c < 0x20) sb.append(String.format("\\u%04x", (int) c));
                else sb.append(c);
            }
            sb.append('"');
        }

        private static byte[] serialize(int[] counters) {
            ByteBuffer buf = ByteBuffer.allocate(8 + counters.length * 4).order(ByteOrder.LITTLE_ENDIAN);
            buf.putInt(CMS_WIDTH);
            buf.putInt(CMS_DEPTH);
            for (int c : counters) buf.putInt(c);
            return buf.array();
        }
    }

    // ===== Hash functions (mirror topk_common) =====
    static long epochMinute(long tsSeconds) {
        return Math.floorDiv(tsSeconds, 60L);
    }

    static long itemHash64(byte[] bytes) {
        long h = 0xCBF29CE484222325L;
        for (byte b : bytes) {
            h ^= (b & 0xFFL);
            h *= 0x100000001B3L;
        }
        return splitmix64(h);
    }

    static long splitmix64(long x) {
        x += 0x9E3779B97F4A7C15L;
        x = (x ^ (x >>> 30)) * 0xBF58476D1CE4E5B9L;
        x = (x ^ (x >>> 27)) * 0x94D049BB133111EBL;
        return x ^ (x >>> 31);
    }

    static int rowHash(int row, long x64) {
        long a = Long.remainderUnsigned(HASH_A[row], MERSENNE_P);
        long b = Long.remainderUnsigned(HASH_B[row], MERSENNE_P);
        long x = Long.remainderUnsigned(x64, MERSENNE_P);
        long prod = mulmodP61(a, x);
        long sum = (prod + b) % MERSENNE_P;
        return (int) Long.remainderUnsigned(sum, CMS_WIDTH);
    }

    static long mulmodP61(long a, long b) {
        // Java lacks u128; use BigInteger only on overflow-risk path.
        // For a, b < 2^61 we can use Math.multiplyHigh-based reduction, but
        // BigInteger keeps this demo readable and is only called per-hash per-row.
        return java.math.BigInteger.valueOf(a)
            .multiply(java.math.BigInteger.valueOf(b))
            .mod(java.math.BigInteger.valueOf(MERSENNE_P))
            .longValue();
    }
}
