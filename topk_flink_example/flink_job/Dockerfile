FROM maven:3.9-eclipse-temurin-11 AS build
WORKDIR /src
COPY pom.xml .
RUN mvn -B -q dependency:go-offline || true
COPY src ./src
RUN mvn -B -q -DskipTests package

FROM flink:1.18.1-scala_2.12-java11
COPY --from=build /src/target/topk-flink-job.jar /opt/flink/usrlib/topk-flink-job.jar
