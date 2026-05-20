//! 搜索过滤器模块

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

/// 排序顺序
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// 降序
    #[default]
    Desc,
    /// 升序
    Asc,
}

/// 排序字段
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    /// 创建时间
    #[default]
    CreatedAt,
    /// 更新时间
    UpdatedAt,
    /// 相关性
    Relevance,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

impl DateRange {
    pub fn new(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Self {
        Self { from, to }
    }

    /// 创建最近 N 天的范围
    pub fn last_days(days: i64) -> Self {
        let to = Utc::now();
        let from = to - chrono::Duration::days(days);
        Self {
            from: Some(from),
            to: Some(to),
        }
    }

    /// 创建本周范围
    pub fn this_week() -> Self {
        let now = Utc::now();
        let start_of_week =
            now - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);
        Self {
            from: Some(start_of_week),
            to: Some(now),
        }
    }

    /// 创建本月范围
    pub fn this_month() -> Self {
        let now = Utc::now();
        let start_of_month = now - chrono::Duration::days(now.day0() as i64);
        Self {
            from: Some(start_of_month),
            to: Some(now),
        }
    }

    /// 创建今年范围
    pub fn this_year() -> Self {
        let now = Utc::now();
        let start_of_year = now.with_month(1).unwrap().with_day(1).unwrap();
        Self {
            from: Some(start_of_year),
            to: Some(now),
        }
    }
}

/// 记忆过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryFilter {
    /// 标签过滤
    pub tags: Option<Vec<String>>,

    /// 记忆类型
    pub memory_type: Option<Vec<String>>,

    /// 日期范围
    pub date_range: Option<DateRange>,

    /// 共享状态
    pub is_shared: Option<bool>,

    /// 有无媒体文件
    pub has_media: Option<bool>,
}

impl MemoryFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_types(mut self, types: Vec<String>) -> Self {
        self.memory_type = Some(types);
        self
    }

    pub fn with_date_range(mut self, range: DateRange) -> Self {
        self.date_range = Some(range);
        self
    }

    pub fn shared_only(mut self) -> Self {
        self.is_shared = Some(true);
        self
    }

    pub fn with_media(mut self) -> Self {
        self.has_media = Some(true);
        self
    }
}

/// 高级搜索选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchOptions {
    /// 模糊匹配
    pub fuzzy: bool,

    /// 包含标题搜索
    pub include_title: bool,

    /// 包含内容搜索
    pub include_content: bool,

    /// 精确匹配
    pub exact_match: bool,
}

impl Default for AdvancedSearchOptions {
    fn default() -> Self {
        Self {
            fuzzy: true,
            include_title: true,
            include_content: true,
            exact_match: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_order_default() {
        assert_eq!(SortOrder::default(), SortOrder::Desc);
    }

    #[test]
    fn test_sort_field_default() {
        assert_eq!(SortField::default(), SortField::CreatedAt);
    }

    #[test]
    fn test_date_range_this_week() {
        let range = DateRange::this_week();
        assert!(range.from.is_some());
        assert!(range.to.is_some());
    }

    #[test]
    fn test_date_range_this_month() {
        let range = DateRange::this_month();
        assert!(range.from.is_some());
        assert!(range.to.is_some());

        let from = range.from.unwrap();
        let to = range.to.unwrap();
        assert!(from <= to);
    }

    #[test]
    fn test_memory_filter_builder() {
        let filter = MemoryFilter::new()
            .with_tags(vec!["旅行".to_string(), "美食".to_string()])
            .with_types(vec!["text".to_string()])
            .shared_only()
            .with_media();

        assert!(filter.tags.is_some());
        assert!(filter.memory_type.is_some());
        assert_eq!(filter.is_shared, Some(true));
        assert_eq!(filter.has_media, Some(true));
    }

    #[test]
    fn test_advanced_search_options_default() {
        let options = AdvancedSearchOptions::default();
        assert!(options.fuzzy);
        assert!(options.include_title);
        assert!(options.include_content);
        assert!(!options.exact_match);
    }
}
