//! 爬虫模块
//! 
//! 【设计模式：门面模式（Facade Pattern）】
//! 此模块作为爬虫子系统的统一入口，对外隐藏内部实现细节
//! 使用pub use重新导出各个爬虫类型，简化外部调用

// 【知识点：模块声明与导出】
// pub mod 声明子模块并使其对外可见
// 模块查找顺序：
// 1. homepage.rs 文件
// 2. homepage/mod.rs 文件
// 3. homepage/ 目录下的定义
pub mod homepage;
pub mod ranking;
pub mod search;
pub mod download;

// 【知识点：pub use 重导出】
// 将子模块中的类型提升到当前模块，简化导入路径
// 外部可以这样使用：use crate::crawlers::HomepageCrawler;
// 而不是：use crate::crawlers::homepage::HomepageCrawler;
//
// 这是Rust的惯用法，称为"统一接口"或"扁平化导出"
pub use homepage::HomepageCrawler;
pub use ranking::RankingCrawler;
pub use search::SearchCrawler;
pub use download::DownloadCrawler;

// 【扩展知识：模块组织最佳实践】
//
// 1. 模块分层：
//    mod.rs 文件只负责组织和导出，不包含业务逻辑
//    业务代码放在同名rs文件或子目录中
//
// 2. 可见性控制：
//    - pub: 完全公开
//    - pub(crate): 仅crate内部可见
//    - pub(super): 仅父模块可见
//    - 默认: 仅当前模块可见
//
// 3. 条件编译（示例）：
//    #[cfg(feature = "advanced-crawler")]
//    pub mod advanced;
//    根据feature标志条件编译模块
//
// 4. 文档测试（示例）：
//    /// ```
//    /// use gequ::crawlers::HomepageCrawler;
//    /// let crawler = HomepageCrawler::new(None, None, None);
//    /// ```
//    在文档中编写可执行的代码示例
