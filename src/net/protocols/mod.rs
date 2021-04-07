/// Protocol for address and get-address messages. Implements how nodes exchange
/// connection information about other nodes on the network. Address and
/// get-address messages are exchanged continually alongside ping-pong messages
/// as part of a network connection.
///
/// Protocol starts by creating a subscription to address and get address
/// messages. Then the protocol sends out a get address message and waits for an
/// address message. Upon receiving an address messages, nodes add the
/// address information to their local store.
pub mod protocol_address;

/// Manages the tasks for the network protocol. Used by other connection
/// protocols to handle asynchronous task execution across the network. Runs all
/// tasks that are handed to it on an executor that has stopping functionality.
pub mod protocol_jobs_manager;

/// Protocol for ping-pong keep-alive messages. Implements ping message and pong
/// response. These messages are like the network heartbeat- they are sent
/// continually between nodes, to ensure each node is still alive and active.
/// Ping-pong messages ensure that the network doesn't
/// time out.
///
/// Protocol starts by creating a subscription to ping and pong messages. Then
/// it starts a loop with a timer and runs ping-pong in the task manager. It
/// sends out a ping and waits for pong reply. Then waits for ping and replies
/// with a pong.
pub mod protocol_ping;

/// Seed server protocol. Seed server is used when connecting to the network for
/// the first time. Returns a list of IP addresses that nodes can connect to.
///
/// To start the seed protocol, we create a subscription to the address message,
/// and send our address to the seed server. Then we send a get-address message
/// and receive an address message. We add these addresses to our internal
/// store.
pub mod protocol_seed;

/// Protocol for version information handshake between nodes at the start of a
/// connection. Implements the process for exchanging version information
/// between nodes. This is the first step when establishing a p2p connection.
///
/// The version protocol starts of by instantiating the protocol and creating a
/// new subscription to version and version acknowledgement messages. Then we
/// run the protocol. Nodes send a version message and wait for a version
/// acknowledgement, while asynchronously waiting for version info from the
/// other node and sending the version acknowledgement.
pub mod protocol_version;

pub use protocol_address::ProtocolAddress;
pub use protocol_jobs_manager::{ProtocolJobsManager, ProtocolJobsManagerPtr};
pub use protocol_ping::ProtocolPing;
pub use protocol_seed::ProtocolSeed;
pub use protocol_version::ProtocolVersion;
