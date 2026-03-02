use std::sync::Arc;

use tokio::runtime::Runtime;

pub fn build() -> Result<Arc<Runtime>, std::io::Error> {
    Runtime::new().map(Arc::new)
}
