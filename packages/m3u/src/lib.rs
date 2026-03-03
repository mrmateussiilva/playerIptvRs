//! M3U/M3U8 em streaming: leitura incremental (arquivo ou URL com auth)
//! e parsing que emite itens por canal para a UI não bloquear.

pub mod model;
pub mod parser;
pub mod source;

pub use model::{ChannelItem, ExtInfMeta};
pub use parser::M3uIncrementalParser;
pub use source::{load_m3u_from_file, load_m3u_from_url, Progress, RequestOptions};
