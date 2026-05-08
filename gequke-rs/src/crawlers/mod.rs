pub mod homepage;
pub mod ranking;
pub mod search;
pub mod download;

pub use homepage::HomepageCrawler;
pub use ranking::RankingCrawler;
pub use search::SearchCrawler;
pub use download::DownloadCrawler;