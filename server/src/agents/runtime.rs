use std::sync::Arc;

use wasmtime::{Engine, Linker, Module, ResourceLimiter, Store};

use crate::error::AppError;

/// Default memory limit per agent instance (64 MB).
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
/// Default fuel per agent invocation (prevents infinite loops).
const DEFAULT_FUEL: u64 = 1_000_000_000;

/// Host state threaded through each WASM store.
pub struct HostState {
    /// Output accumulated by the agent (via host logging calls).
    pub output: Vec<String>,
}

impl ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(desired <= MEMORY_LIMIT_BYTES)
    }

    fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// Singleton wasmtime engine — thread-safe, shared across all agent instances.
#[derive(Clone)]
pub struct WasmRuntime {
    engine: Arc<Engine>,
}

impl WasmRuntime {
    /// Initialise the runtime with safe production defaults.
    pub fn new() -> Result<Self, AppError> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)
            .map_err(|e| AppError::Internal(format!("wasmtime engine init: {e}")))?;

        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    /// Load WASM bytes into a compiled module and return an AgentInstance.
    pub fn load(&self, wasm_bytes: &[u8]) -> Result<AgentInstance, AppError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| AppError::BadRequest(format!("invalid WASM: {e}")))?;

        Ok(AgentInstance {
            engine: self.engine.clone(),
            module,
        })
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("WasmRuntime::new should not fail with valid config")
    }
}

/// A compiled WASM agent module ready to instantiate and call.
pub struct AgentInstance {
    engine: Arc<Engine>,
    module: Module,
}

impl AgentInstance {
    /// Call a named export function with no args, returning the collected output.
    pub fn call(&self, fn_name: &str) -> Result<String, AppError> {
        let mut store = Store::new(
            &self.engine,
            HostState {
                output: Vec::new(),
            },
        );

        // Apply fuel limit to prevent infinite loops.
        store
            .set_fuel(DEFAULT_FUEL)
            .map_err(|e| AppError::Internal(format!("set_fuel: {e}")))?;

        // Apply memory limiter via the HostState ResourceLimiter impl.
        store.limiter(|state| state as &mut dyn ResourceLimiter);

        let mut linker: Linker<HostState> = Linker::new(&self.engine);

        // Host logging function: `env::lore_log(ptr: i32, len: i32)`
        linker
            .func_wrap(
                "env",
                "lore_log",
                |mut caller: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| {
                    let mem = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(m)) => m,
                        _ => return,
                    };
                    let data = mem.data(&caller);
                    let start = ptr as usize;
                    let end = start.saturating_add(len as usize);
                    if end <= data.len() {
                        if let Ok(s) = std::str::from_utf8(&data[start..end]) {
                            caller.data_mut().output.push(s.to_string());
                        }
                    }
                },
            )
            .map_err(|e| AppError::Internal(format!("linker.func_wrap: {e}")))?;

        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| AppError::Internal(format!("instantiate: {e}")))?;

        let func = instance
            .get_func(&mut store, fn_name)
            .ok_or_else(|| AppError::NotFound(format!("WASM export not found: {fn_name}")))?;

        func.call(&mut store, &[], &mut [])
            .map_err(|e| AppError::Internal(format!("WASM call '{fn_name}': {e}")))?;

        let output = store.data().output.join("\n");
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_new() {
        let rt = WasmRuntime::new();
        assert!(rt.is_ok(), "WasmRuntime::new() should succeed");
    }

    #[test]
    fn test_load_invalid_wasm() {
        let rt = WasmRuntime::new().unwrap();
        let result = rt.load(b"not wasm bytes");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_valid_wasm() {
        let wat = r#"
            (module
              (import "env" "lore_log" (func $lore_log (param i32 i32)))
              (func (export "run"))
            )
        "#;
        let wasm = wat::parse_str(wat).expect("valid WAT");
        let rt = WasmRuntime::new().unwrap();
        let instance = rt.load(&wasm);
        assert!(instance.is_ok(), "should load valid WASM");
    }

    #[test]
    fn test_call_run_function() {
        let wat = r#"
            (module
              (import "env" "lore_log" (func $lore_log (param i32 i32)))
              (func (export "run"))
            )
        "#;
        let wasm = wat::parse_str(wat).expect("valid WAT");
        let rt = WasmRuntime::new().unwrap();
        let instance = rt.load(&wasm).unwrap();
        let result = instance.call("run");
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_missing_export() {
        let wat = r#"
            (module
              (import "env" "lore_log" (func $lore_log (param i32 i32)))
              (func (export "run"))
            )
        "#;
        let wasm = wat::parse_str(wat).expect("valid WAT");
        let rt = WasmRuntime::new().unwrap();
        let instance = rt.load(&wasm).unwrap();
        let result = instance.call("nonexistent");
        assert!(result.is_err());
    }
}
