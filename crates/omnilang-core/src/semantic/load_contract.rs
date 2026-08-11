//! 批次 4 数据能力语义契约常量层（登记先行——ADR-074）。
//! 任何新 @load 派生信号/模式词必须在此登记，且同时更新 universe 域 9/13 登记册
//! 与 vocabulary 域 9（scripts/verify-*.py 双向断言，漂移即门禁失败）。

/// @load 匿名派生信号保留名全集（spec §2.1 信号表；含新增 reset）。
/// 编译器保留名判定（V-F8 先例）——匿名实例派生信号集合，禁止用户同名声明。
pub const LOAD_RESERVED_NAMES: &[&str] = &["loading", "error", "data", "retry", "reset"];
/// 命名分页派生信号后缀（spec §2.2 信号表）——`{name}` 实例派生
/// `{name}Page`/`{name}HasMore`/`{name}LoadingMore`/`{name}LoadMore`。
pub const LOAD_PAGINATED_SUFFIXES: &[&str] = &["Page", "HasMore", "LoadingMore", "LoadMore"];
/// @load 局部模式词（spec §2.2/§2.3/§2.4 BNF）——`paginated`/`scroll`/`interval`。
pub const LOAD_PARAM_WORDS: &[&str] = &["paginated", "scroll", "interval"];
/// std/timing 时间函数（spec §2.5）——@load 轮询参数与 std 事件可组合的节流函数。
pub const TIMING_FUNCS: &[&str] = &["debounce", "throttle"];
/// 分页内建信封类型名（spec §2.2）——`Page<T> { items, hasMore }`。
pub const PAGE_TYPE_NAME: &str = "Page";

#[cfg(test)]
mod tests {
    use super::*;

    /// 批次 4 语义契约锚点——verify 脚本经 rust_str_array 提取断言（ADR-074）。
    /// 任何新 @load 派生信号/模式词必须在此登记，且同时更新 universe 域 9/13 登记册。
    #[test]
    fn load_contract_matches_design() {
        // spec §2.1 匿名派生信号（含新增 reset）
        assert_eq!(
            LOAD_RESERVED_NAMES,
            &["loading", "error", "data", "retry", "reset"]
        );
        // spec §2.2 分页派生信号后缀
        assert_eq!(
            LOAD_PAGINATED_SUFFIXES,
            &["Page", "HasMore", "LoadingMore", "LoadMore"]
        );
        // spec §2.2/2.3/2.4 @load 模式词
        assert_eq!(LOAD_PARAM_WORDS, &["paginated", "scroll", "interval"]);
        // spec §2.5 std/timing
        assert_eq!(TIMING_FUNCS, &["debounce", "throttle"]);
        assert_eq!(PAGE_TYPE_NAME, "Page");
    }
}
