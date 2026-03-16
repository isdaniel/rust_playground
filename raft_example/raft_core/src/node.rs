use std::collections::HashMap;
use std::time::Duration;

use rand::RngExt as _;
use tokio::sync::mpsc;
use tokio::time::{self, Instant};
use tracing::{debug, info, warn};

use crate::config::{NodeId, RaftConfig};
use crate::error::Result;
use crate::log::LogEntry;
use crate::rpc::*;
use crate::state_machine::StateMachine;
use crate::storage::{PersistentState, Storage};
use crate::transport::{self, oneshot};

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Follower,
    Candidate,
    Leader,
}

// ---------------------------------------------------------------------------
// Pending client request (waiting for commit)
// ---------------------------------------------------------------------------

struct PendingRequest {
    log_index: u64,
    resp_tx: oneshot::Sender<RpcMessage>,
}

// ---------------------------------------------------------------------------
// RaftNode -- the core consensus engine, generic over a StateMachine
// ---------------------------------------------------------------------------

pub struct RaftNode<S: StateMachine> {
    // -- configuration --
    config: RaftConfig,

    // -- persistent state (Raft paper Figure 2) --
    current_term: u64,
    voted_for: Option<NodeId>,
    log: Vec<LogEntry>, // 0-indexed internally; index = entry.index - 1

    // -- volatile state on all servers --
    commit_index: u64,
    last_applied: u64,

    // -- volatile state on leaders --
    next_index: HashMap<NodeId, u64>,
    match_index: HashMap<NodeId, u64>,

    // -- role --
    role: Role,
    leader_id: Option<NodeId>,
    votes_received: u64,

    // -- subsystems --
    storage: Storage,
    state_machine: S,

    // -- communication --
    /// Incoming RPCs from the transport layer.
    rpc_rx: mpsc::Receiver<(RpcMessage, oneshot::Sender<RpcMessage>)>,

    // -- pending client requests awaiting commit --
    pending_requests: Vec<PendingRequest>,
}

impl<S: StateMachine> RaftNode<S> {
    pub fn new(
        config: RaftConfig,
        storage: Storage,
        state_machine: S,
        rpc_rx: mpsc::Receiver<(RpcMessage, oneshot::Sender<RpcMessage>)>,
    ) -> Result<Self> {
        let persistent = storage.load()?;
        info!(
            "node {} loaded state: term={}, voted_for={:?}, log_len={}",
            config.id,
            persistent.current_term,
            persistent.voted_for,
            persistent.log.len()
        );
        Ok(Self {
            config,
            current_term: persistent.current_term,
            voted_for: persistent.voted_for,
            log: persistent.log,
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            role: Role::Follower,
            leader_id: None,
            votes_received: 0,
            storage,
            state_machine,
            rpc_rx,
            pending_requests: Vec::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Main event loop
    // -----------------------------------------------------------------------

    pub async fn run(mut self) -> Result<()> {
        let mut election_deadline = self.new_election_deadline();

        loop {
            let heartbeat_interval =
                Duration::from_millis(self.config.heartbeat_interval_ms);

            tokio::select! {
                // -- incoming RPC --
                Some((msg, resp_tx)) = self.rpc_rx.recv() => {
                    let resp = self.handle_rpc(msg).await;
                    let _ = resp_tx.send(resp);
                    // Any valid RPC from current leader resets election timer
                    if self.role == Role::Follower {
                        election_deadline = self.new_election_deadline();
                    }
                }

                // -- election timeout (followers & candidates) --
                _ = time::sleep_until(election_deadline), if self.role != Role::Leader => {
                    info!("node {}: election timeout, starting election", self.config.id);
                    self.start_election().await;
                    election_deadline = self.new_election_deadline();
                }

                // -- heartbeat tick (leader only) --
                _ = time::sleep(heartbeat_interval), if self.role == Role::Leader => {
                    self.send_append_entries_to_all().await;
                }
            }

            // Apply committed entries to the state machine.
            self.apply_committed_entries();
        }
    }

    // -----------------------------------------------------------------------
    // Election
    // -----------------------------------------------------------------------

    async fn start_election(&mut self) {
        self.role = Role::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.config.id);
        self.votes_received = 1; // vote for self
        self.leader_id = None;
        self.persist();

        info!(
            "node {} is candidate for term {}",
            self.config.id, self.current_term
        );

        let req = RpcMessage::RequestVoteRequest(RequestVoteRequest {
            term: self.current_term,
            candidate_id: self.config.id,
            last_log_index: self.last_log_index(),
            last_log_term: self.last_log_term(),
        });

        let peers: Vec<_> = self.config.other_peers().into_iter().cloned().collect();
        let term_snapshot = self.current_term;

        // Collect vote responses (send in parallel, collect results).
        let mut vote_results = Vec::new();
        for peer in &peers {
            let addr = peer.addr;
            let req_clone = req.clone();
            vote_results.push(tokio::spawn(async move {
                let result = tokio::time::timeout(
                    Duration::from_millis(500),
                    transport::rpc_call(addr, &req_clone),
                )
                .await;
                (addr, result)
            }));
        }

        for handle in vote_results {
            if let Ok((_addr, Ok(Ok(resp)))) = handle.await {
                self.handle_vote_response(resp, term_snapshot);
            } else {
                debug!("vote request to a peer failed or timed out");
            }
            // If we already won, stop waiting.
            if self.role == Role::Leader {
                break;
            }
        }

        // Immediately assert authority by sending heartbeats so that
        // followers reset their election timers right away.
        if self.role == Role::Leader {
            self.send_append_entries_to_all().await;
        }
    }

    fn handle_vote_response(&mut self, msg: RpcMessage, expected_term: u64) {
        if self.role != Role::Candidate || self.current_term != expected_term {
            return;
        }
        if let RpcMessage::RequestVoteResponse(resp) = msg {
            if resp.term > self.current_term {
                self.become_follower(resp.term);
                return;
            }
            if resp.vote_granted {
                self.votes_received += 1;
                info!(
                    "node {} received vote from {} ({}/{})",
                    self.config.id,
                    resp.from,
                    self.votes_received,
                    self.config.quorum()
                );
                if self.votes_received as usize >= self.config.quorum() {
                    self.become_leader();
                }
            }
        }
    }

    fn become_leader(&mut self) {
        info!(
            "node {} became LEADER for term {}",
            self.config.id, self.current_term
        );
        self.role = Role::Leader;
        self.leader_id = Some(self.config.id);
        // Initialize next_index and match_index for each peer.
        let next = self.last_log_index() + 1;
        for peer in self.config.other_peers() {
            self.next_index.insert(peer.id, next);
            self.match_index.insert(peer.id, 0);
        }
        // Append a no-op entry to commit entries from previous terms
        // (Raft paper Section 5.4.2).
        let noop = LogEntry {
            term: self.current_term,
            index: self.last_log_index() + 1,
            command: None, // None == Noop
        };
        self.log.push(noop);
        self.persist();
    }

    fn become_follower(&mut self, term: u64) {
        debug!(
            "node {} stepping down to follower (term {} -> {})",
            self.config.id, self.current_term, term
        );
        self.role = Role::Follower;
        self.current_term = term;
        self.voted_for = None;
        self.votes_received = 0;
        self.persist();
    }

    // -----------------------------------------------------------------------
    // RPC dispatch
    // -----------------------------------------------------------------------

    async fn handle_rpc(&mut self, msg: RpcMessage) -> RpcMessage {
        match msg {
            RpcMessage::AppendEntriesRequest(req) => {
                let resp = self.handle_append_entries(req);
                RpcMessage::AppendEntriesResponse(resp)
            }
            RpcMessage::RequestVoteRequest(req) => {
                let resp = self.handle_request_vote(req);
                RpcMessage::RequestVoteResponse(resp)
            }
            RpcMessage::ClientRequest(req) => {
                let resp = self.handle_client_request(req).await;
                RpcMessage::ClientResponse(resp)
            }
            other => {
                warn!("unexpected RPC: {:?}", other);
                RpcMessage::ClientResponse(ClientResponse {
                    request_id: 0,
                    result: ClientResult::Error {
                        message: "unexpected message".into(),
                    },
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // AppendEntries handler  (Raft paper Figure 2)
    // -----------------------------------------------------------------------

    fn handle_append_entries(&mut self, req: AppendEntriesRequest) -> AppendEntriesResponse {
        // 1. Reply false if term < currentTerm.
        if req.term < self.current_term {
            return AppendEntriesResponse {
                term: self.current_term,
                success: false,
                from: self.config.id,
                last_log_index: self.last_log_index(),
            };
        }

        // If RPC term >= currentTerm, recognise sender as leader.
        if req.term > self.current_term || self.role != Role::Follower {
            self.become_follower(req.term);
        }
        self.leader_id = Some(req.leader_id);

        // 2. Reply false if log doesn't contain an entry at prevLogIndex
        //    whose term matches prevLogTerm.
        if req.prev_log_index > 0 {
            match self.log.get((req.prev_log_index - 1) as usize) {
                Some(entry) if entry.term != req.prev_log_term => {
                    // 3. Conflict: delete the entry and all that follow it.
                    self.log.truncate((req.prev_log_index - 1) as usize);
                    self.persist();
                    return AppendEntriesResponse {
                        term: self.current_term,
                        success: false,
                        from: self.config.id,
                        last_log_index: self.last_log_index(),
                    };
                }
                None => {
                    return AppendEntriesResponse {
                        term: self.current_term,
                        success: false,
                        from: self.config.id,
                        last_log_index: self.last_log_index(),
                    };
                }
                _ => {}
            }
        }

        // 4. Append any new entries not already in the log.
        for entry in &req.entries {
            let idx = (entry.index - 1) as usize;
            if idx < self.log.len() {
                if self.log[idx].term != entry.term {
                    self.log.truncate(idx);
                    self.log.push(entry.clone());
                }
                // else: already have this entry, skip.
            } else {
                self.log.push(entry.clone());
            }
        }

        // 5. If leaderCommit > commitIndex, set commitIndex.
        if req.leader_commit > self.commit_index {
            self.commit_index = std::cmp::min(req.leader_commit, self.last_log_index());
        }

        self.persist();

        AppendEntriesResponse {
            term: self.current_term,
            success: true,
            from: self.config.id,
            last_log_index: self.last_log_index(),
        }
    }

    // -----------------------------------------------------------------------
    // RequestVote handler  (Raft paper Figure 2)
    // -----------------------------------------------------------------------

    fn handle_request_vote(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        // 1. Reply false if term < currentTerm.
        if req.term < self.current_term {
            return RequestVoteResponse {
                term: self.current_term,
                vote_granted: false,
                from: self.config.id,
            };
        }

        if req.term > self.current_term {
            self.become_follower(req.term);
        }

        // 2. If votedFor is null or candidateId, and candidate's log is at
        //    least as up-to-date as receiver's log, grant vote.
        let can_vote =
            self.voted_for.is_none() || self.voted_for == Some(req.candidate_id);
        let log_ok = self.is_log_up_to_date(req.last_log_term, req.last_log_index);

        if can_vote && log_ok {
            self.voted_for = Some(req.candidate_id);
            self.persist();
            info!(
                "node {} granted vote to {} for term {}",
                self.config.id, req.candidate_id, self.current_term
            );
            RequestVoteResponse {
                term: self.current_term,
                vote_granted: true,
                from: self.config.id,
            }
        } else {
            RequestVoteResponse {
                term: self.current_term,
                vote_granted: false,
                from: self.config.id,
            }
        }
    }

    /// Raft paper Section 5.4.1: compare last log entry.
    fn is_log_up_to_date(&self, candidate_last_term: u64, candidate_last_index: u64) -> bool {
        let my_last_term = self.last_log_term();
        let my_last_index = self.last_log_index();
        if candidate_last_term != my_last_term {
            candidate_last_term > my_last_term
        } else {
            candidate_last_index >= my_last_index
        }
    }

    // -----------------------------------------------------------------------
    // Leader: send AppendEntries to all peers
    // -----------------------------------------------------------------------

    async fn send_append_entries_to_all(&mut self) {
        let peers: Vec<_> = self.config.other_peers().into_iter().cloned().collect();
        let mut handles = Vec::new();

        for peer in &peers {
            let next = *self.next_index.get(&peer.id).unwrap_or(&1);
            let prev_log_index = next - 1;
            let prev_log_term = if prev_log_index > 0 {
                self.log
                    .get((prev_log_index - 1) as usize)
                    .map(|e| e.term)
                    .unwrap_or(0)
            } else {
                0
            };

            let entries: Vec<LogEntry> = self
                .log
                .iter()
                .filter(|e| e.index >= next)
                .cloned()
                .collect();

            let req = RpcMessage::AppendEntriesRequest(AppendEntriesRequest {
                term: self.current_term,
                leader_id: self.config.id,
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit: self.commit_index,
            });

            let addr = peer.addr;
            let peer_id = peer.id;
            handles.push(tokio::spawn(async move {
                let result = tokio::time::timeout(
                    Duration::from_millis(500),
                    transport::rpc_call(addr, &req),
                )
                .await;
                (peer_id, result)
            }));
        }

        for handle in handles {
            if let Ok((peer_id, Ok(Ok(RpcMessage::AppendEntriesResponse(resp))))) =
                handle.await
            {
                if resp.term > self.current_term {
                    self.become_follower(resp.term);
                    return;
                }
                if resp.success {
                    // Update nextIndex and matchIndex for this peer.
                    let new_match = resp.last_log_index;
                    self.match_index.insert(peer_id, new_match);
                    self.next_index.insert(peer_id, new_match + 1);
                } else {
                    // Decrement nextIndex and retry on next heartbeat.
                    let ni = self.next_index.entry(peer_id).or_insert(1);
                    if *ni > 1 {
                        // Fast back-up: jump to the follower's last log index + 1.
                        *ni = std::cmp::min(*ni - 1, resp.last_log_index + 1);
                    }
                }
            }
        }

        // Advance commit_index if a majority has replicated.
        self.advance_commit_index();
    }

    /// Raft paper: If there exists an N such that N > commitIndex, a majority
    /// of matchIndex[i] >= N, and log[N].term == currentTerm, set commitIndex = N.
    fn advance_commit_index(&mut self) {
        let old = self.commit_index;
        for n in (self.commit_index + 1)..=self.last_log_index() {
            let idx = (n - 1) as usize;
            if idx >= self.log.len() {
                break;
            }
            if self.log[idx].term != self.current_term {
                continue;
            }
            // Count replicas (self + peers with matchIndex >= n).
            let mut count: usize = 1; // self
            for peer in self.config.other_peers() {
                if *self.match_index.get(&peer.id).unwrap_or(&0) >= n {
                    count += 1;
                }
            }
            if count >= self.config.quorum() {
                self.commit_index = n;
            }
        }
        if self.commit_index > old {
            debug!(
                "node {} commit_index advanced {} -> {}",
                self.config.id, old, self.commit_index
            );
        }
    }

    // -----------------------------------------------------------------------
    // Apply committed entries to the state machine
    // -----------------------------------------------------------------------

    fn apply_committed_entries(&mut self) {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            let idx = (self.last_applied - 1) as usize;
            if let Some(entry) = self.log.get(idx) {
                debug!(
                    "node {} applying index {} (term {})",
                    self.config.id, entry.index, entry.term
                );
                let result = self.state_machine.apply(&entry.command);

                // Resolve any pending client request waiting on this index.
                self.resolve_pending(entry.index, result);
            }
        }
    }

    fn resolve_pending(&mut self, log_index: u64, value: Option<Vec<u8>>) {
        if let Some(pos) = self
            .pending_requests
            .iter()
            .position(|p| p.log_index == log_index)
        {
            let pending = self.pending_requests.remove(pos);
            let _ = pending.resp_tx.send(RpcMessage::ClientResponse(ClientResponse {
                request_id: 0,
                result: ClientResult::Ok { value },
            }));
        }
    }

    // -----------------------------------------------------------------------
    // Client request handling (leader only)
    // -----------------------------------------------------------------------

    async fn handle_client_request(&mut self, req: ClientRequest) -> ClientResponse {
        // Read-only Query can be served without log replication.
        // (Linearizable reads would need a read-index protocol; for
        // simplicity we read from committed state on the leader.)
        if let ClientCommand::Query { ref payload } = req.command {
            if self.role != Role::Leader {
                return ClientResponse {
                    request_id: req.request_id,
                    result: ClientResult::NotLeader {
                        leader_addr: self.leader_addr_string(),
                    },
                };
            }
            let value = self.state_machine.query(payload);
            return ClientResponse {
                request_id: req.request_id,
                result: ClientResult::Ok { value },
            };
        }

        // Writes must go through the leader.
        if self.role != Role::Leader {
            return ClientResponse {
                request_id: req.request_id,
                result: ClientResult::NotLeader {
                    leader_addr: self.leader_addr_string(),
                },
            };
        }

        // Extract the payload from the Mutate command.
        let payload = match req.command {
            ClientCommand::Mutate { payload } => payload,
            ClientCommand::Query { .. } => unreachable!(),
        };

        let entry = LogEntry {
            term: self.current_term,
            index: self.last_log_index() + 1,
            command: Some(payload),
        };
        let log_index = entry.index;
        self.log.push(entry);
        self.persist();

        // We will wait (with a timeout) for this entry to be committed.
        let (resp_tx, resp_rx) = oneshot::channel();
        self.pending_requests.push(PendingRequest {
            log_index,
            resp_tx,
        });

        // Immediately replicate to peers.
        self.send_append_entries_to_all().await;

        // Apply any entries that were just committed so the pending request
        // can be resolved immediately.
        self.apply_committed_entries();

        // Wait for commit (timeout after a few seconds).
        match tokio::time::timeout(Duration::from_secs(5), resp_rx).await {
            Ok(Ok(resp)) => {
                if let RpcMessage::ClientResponse(cr) = resp {
                    cr
                } else {
                    ClientResponse {
                        request_id: req.request_id,
                        result: ClientResult::Error {
                            message: "internal error".into(),
                        },
                    }
                }
            }
            _ => {
                // Clean up the pending request.
                self.pending_requests
                    .retain(|p| p.log_index != log_index);
                ClientResponse {
                    request_id: req.request_id,
                    result: ClientResult::Error {
                        message: "timeout waiting for commit".into(),
                    },
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn last_log_index(&self) -> u64 {
        self.log.last().map(|e| e.index).unwrap_or(0)
    }

    fn last_log_term(&self) -> u64 {
        self.log.last().map(|e| e.term).unwrap_or(0)
    }

    fn persist(&self) {
        let state = PersistentState {
            current_term: self.current_term,
            voted_for: self.voted_for,
            log: self.log.clone(),
        };
        if let Err(e) = self.storage.save(&state) {
            warn!("failed to persist state: {}", e);
        }
    }

    fn new_election_deadline(&self) -> Instant {
        let mut rng = rand::rng();
        let ms = rng.random_range(
            self.config.election_timeout_min_ms..=self.config.election_timeout_max_ms,
        );
        Instant::now() + Duration::from_millis(ms)
    }

    fn leader_addr_string(&self) -> Option<String> {
        self.leader_id.and_then(|id| {
            self.config
                .peers
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.addr.to_string())
        })
    }
}
