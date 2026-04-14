// Shared test utilities

use synaptic_graph::db::Database;

pub fn test_db() -> Database {
    Database::open_in_memory().expect("Failed to create in-memory database")
}
