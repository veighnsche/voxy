use std::sync::Arc;

use tokio::runtime::Runtime;

pub fn build() -> Arc<Runtime> {
    Arc::new(Runtime::new().expect("failed to create tokio runtime"))
}
