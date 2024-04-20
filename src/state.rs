use std::sync::{Arc, Mutex};

use uuid::Uuid;

// TODO: setup real appstate!
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub todos: Arc<Mutex<Uuid>>,
}
