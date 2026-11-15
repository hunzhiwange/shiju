//! 上下文管理模块
//!
//! 本模块提供线程本地的上下文管理功能，类似于 React 的 Context API。
//! 它允许在调用栈中传递数据，而无需显式地将参数传递给每一层函数。
//!
//! # 主要特性
//!
//! - **线程本地存储**：每个线程都有独立的上下文存储，避免跨线程数据竞争
//! - **类型安全**：使用泛型和 `TypeId` 确保类型安全的数据访问
//! - **作用域管理**：使用 RAII 模式自动管理上下文的生命周期
//! - **嵌套支持**：支持上下文值的嵌套，内层值会覆盖外层值
//!
//! # 使用示例
//!
//! ```rust
//! use agent_context::context;
//!
//! // 创建一个字符串类型的上下文
//! let ctx = context::create::<String>("app_name");
//!
//! // 提供上下文值并使用它
//! ctx.provide("VibeWindow".to_string(), || {
//!     if let Ok(value) = ctx.use_value() {
//!         println!("应用名称: {}", value);
//!     }
//! });
//! ```

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

// 线程本地上下文存储
//
// 使用 `TypeId` 作为键，存储该类型的值栈（Vec）。
// 使用栈结构是为了支持嵌套的上下文值，内层值会覆盖外层值。
// 当 `provide` 结束时，值会从栈中弹出，恢复到外层的值。
thread_local! {
    static STORE: RefCell<HashMap<TypeId, Vec<Box<dyn Any>>>> = RefCell::new(HashMap::new());
}

/// 上下文未找到错误
///
/// 当尝试使用 `Context::use_value()` 获取上下文值，
/// 但当前作用域中没有提供该类型的值时，会返回此错误。
///
/// # 字段
///
/// - `name`: 上下文的名称，用于错误信息展示
#[derive(Debug)]
pub struct NotFound {
    // 上下文的名称标识
    pub name: &'static str,
}

impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No context found for {}", self.name)
    }
}

impl std::error::Error for NotFound {}

/// 类型安全的上下文
///
/// `Context<T>` 代表一个特定类型 `T` 的上下文，允许在调用栈中
/// 传递该类型的值，而无需显式地作为函数参数传递。
///
/// # 类型参数
///
/// - `T`: 上下文值的类型，必须满足 `'static` 生命周期约束
///
/// # 特性
///
/// - **类型安全**：每个上下文绑定到特定类型，编译时保证类型安全
/// - **线程本地**：上下文值存储在线程本地存储中，每个线程独立
/// - **作用域限制**：上下文值只在 `provide` 的闭包作用域内有效
/// - **嵌套支持**：支持嵌套调用 `provide`，内层值优先
///
/// # 示例
///
/// ```rust
/// let ctx = context::create::<i32>("counter");
///
/// ctx.provide(42, || {
///     // 在此作用域内，use_value 返回 42
///     assert_eq!(ctx.use_value().unwrap().as_ref(), &42);
///
///     ctx.provide(100, || {
///         // 嵌套作用域，内层值覆盖外层
///         assert_eq!(ctx.use_value().unwrap().as_ref(), &100);
///     });
///
///     // 嵌套作用域结束后，恢复到外层值
///     assert_eq!(ctx.use_value().unwrap().as_ref(), &42);
/// });
///
/// // provide 作用域外，无法获取值
/// assert!(ctx.use_value().is_err());
/// ```
pub struct Context<T: 'static> {
    // 上下文的名称，用于错误信息和调试
    name: &'static str,
    // 类型标记，用于编译时类型检查
    _marker: PhantomData<T>,
}

/// 创建一个新的上下文
///
/// 工厂函数，用于创建指定类型的上下文实例。
///
/// # 类型参数
///
/// - `T`: 上下文值的类型，必须满足 `'static` 生命周期约束
///
/// # 参数
///
/// - `name`: 上下文的名称，用于错误信息和调试。建议使用有意义的名称，
///   如 "database_pool"、"config"、"logger" 等
///
/// # 返回值
///
/// 返回一个 `Context<T>` 实例，可用于提供和使用上下文值
///
/// # 示例
///
/// ```rust
/// // 创建一个数据库连接池的上下文
/// let db_ctx = context::create::<DatabasePool>("database_pool");
///
/// // 创建一个配置的上下文
/// let config_ctx = context::create::<AppConfig>("config");
/// ```
pub fn create<T: 'static>(name: &'static str) -> Context<T> {
    Context { name, _marker: PhantomData }
}

impl<T: 'static> Context<T> {
    // 获取当前作用域中的上下文值
    //
    // 从线程本地存储中获取当前类型的上下文值。如果有多个嵌套的
    // `provide` 调用，返回最内层（最近）提供的值。
    //
    // # 返回值
    //
    // - `Ok(Arc<T>)`: 成功获取上下文值的共享引用
    // - `Err(NotFound)`: 当前作用域中没有提供该类型的值
    //
    // # 示例
    //
    // ```rust
    // let ctx = context::create::<String>("greeting");
    //
    // ctx.provide("Hello".to_string(), || {
    //     match ctx.use_value() {
    //         Ok(value) => println!("问候语: {}", value),
    //         Err(e) => eprintln!("错误: {}", e),
    //     }
    // });
    // ```
    //
    // # 线程安全
    //
    // 此方法是线程安全的，每个线程访问自己的上下文存储。
    // 在一个线程中提供的值，无法在另一个线程中访问。
    pub fn use_value(&self) -> Result<Arc<T>, NotFound> {
        STORE.with(|store| {
            // 获取存储的不可变引用
            let store = store.borrow();

            // 尝试获取该类型的值栈
            let Some(stack) = store.get(&TypeId::of::<T>()) else {
                return Err(NotFound { name: self.name });
            };

            // 获取栈顶元素（最近提供的值）
            let Some(top) = stack.last() else { return Err(NotFound { name: self.name }) };

            // 尝试将 Any 类型转换为具体的 Arc<T> 类型
            let Some(v) = top.downcast_ref::<Arc<T>>() else {
                return Err(NotFound { name: self.name });
            };

            // 返回值的克隆引用
            Ok(v.clone())
        })
    }

    // 在指定作用域内提供上下文值
    //
    // 将值推入上下文存储，执行闭包，然后在闭包结束时自动移除该值。
    // 支持嵌套调用，内层值会覆盖外层值。
    //
    // # 参数
    //
    // - `value`: 要提供的上下文值
    // - `f`: 在上下文值有效期间执行的闭包
    //
    // # 返回值
    //
    // 返回闭包 `f` 的执行结果
    //
    // # 生命周期
    //
    // - 值在调用 `provide` 时被推入栈
    // - 值在闭包 `f` 执行完毕后自动从栈中弹出
    // - 如果栈变空，会从存储中移除该类型的条目以节省内存
    //
    // # 示例
    //
    // ```rust
    // let ctx = context::create::<Config>("config");
    // let config = Config::load();
    //
    // let result = ctx.provide(config, || {
    //     // 在此作用域内可以访问 config
    //     let cfg = ctx.use_value().unwrap();
    //     process_with_config(&cfg)
    // });
    //
    // // 作用域外，config 不再可用
    // assert!(ctx.use_value().is_err());
    // ```
    //
    // # RAII 保证
    //
    // 使用 Guard 模式确保即使闭包 panic，上下文值也会被正确清理。
    pub fn provide<R>(&self, value: T, f: impl FnOnce() -> R) -> R {
        // 将值包装在 Arc 中，以便可以在多个地方共享
        let v = Arc::new(value);

        // 清理守卫，在作用域结束时自动移除上下文值
        // 使用 RAII 模式确保即使 panic 也能正确清理
        struct Guard<T: 'static> {
            _marker: PhantomData<T>,
        }

        impl<T: 'static> Drop for Guard<T> {
            fn drop(&mut self) {
                STORE.with(|store| {
                    let mut store = store.borrow_mut();
                    // 获取该类型的值栈
                    if let Some(stack) = store.get_mut(&TypeId::of::<T>()) {
                        // 弹出栈顶元素
                        stack.pop();
                        // 如果栈为空，移除整个条目以节省内存
                        if stack.is_empty() {
                            store.remove(&TypeId::of::<T>());
                        }
                    }
                });
            }
        }

        // 将值推入线程本地存储的栈中
        STORE.with(|store| {
            let mut store = store.borrow_mut();
            // 如果该类型还没有栈，创建一个新的；否则使用现有栈
            store.entry(TypeId::of::<T>()).or_default().push(Box::new(v) as Box<dyn Any>);
        });

        // 创建守卫，确保在作用域结束时清理
        let _guard = Guard::<T> { _marker: PhantomData };

        // 执行用户提供的闭包
        f()
    }
}
