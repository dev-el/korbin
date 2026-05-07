use std::env;
use std::path::PathBuf;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use roto::{Context, Runtime, Val, library};
use crate::REGISTRY;

struct ScriptPermissions {
    allow_shell: AtomicBool,
}

static PERMISSIONS: ScriptPermissions = ScriptPermissions {
    allow_shell: AtomicBool::new(false),
};

#[derive(Clone, Context)]
pub struct EditorContext {
}

#[derive(Clone, Copy)]
pub struct SetupToken {
    // leaving the struct empty triggers an error in roto compilation
    _dummy_byte: u8
}

pub fn rust_bind(mode: Arc<str>, key: Arc<str>, command: Arc<str>) {
    let mut registry = REGISTRY.lock().unwrap();
    registry.set(&mode, "global", &key, crate::Action::EditorCommand(command.to_string()));
}

pub fn rust_bind_ctx(mode: Arc<str>, context: Arc<str>, key: Arc<str>, command: Arc<str>) {
    let mut registry = REGISTRY.lock().unwrap();
    registry.set(&mode, &context, &key, crate::Action::EditorCommand(command.to_string()));
}

pub fn rust_bind_shell(mode: Arc<str>, key: Arc<str>, cmd: Arc<str>, async_exec: bool) {
    let mut registry = REGISTRY.lock().unwrap();
    registry.set(&mode, "global", &key, crate::Action::ShellCommand { cmd: cmd.to_string(), async_exec });
}

pub fn rust_bind_shell_ctx(mode: Arc<str>, context: Arc<str>, key: Arc<str>, cmd: Arc<str>, async_exec: bool) {
    let mut registry = REGISTRY.lock().unwrap();
    registry.set(&mode, &context, &key, crate::Action::ShellCommand { cmd: cmd.to_string(), async_exec });
}

pub fn rust_exec(cmd: Arc<str>) -> Arc<str> {
    if !PERMISSIONS.allow_shell.load(Ordering::SeqCst) {
        return "Permission Denied: Shell commands not allowed. Enable in setup(t: SetupToken) using t.set_shell_access(b: bool).".into();
    }
    
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&*cmd)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                stdout.to_string().into()
            } else {
                format!("Command failed: {}\n{}", stdout, stderr).into()
            }
        }
        Err(e) => format!("Execution Error: {}", e).into(),
    }
}

pub fn run_config() {
    // Reset permissions before running
    PERMISSIONS.allow_shell.store(false, Ordering::SeqCst);

    let lib = library! {
        #[clone] type SetupToken = Val<SetupToken>;

        impl Val<SetupToken>  {
            fn set_shell_access(self, allow: bool) {
                PERMISSIONS.allow_shell.store(allow, Ordering::SeqCst);
            }
        }

        // Global Bind: Defaults to "global" context
        fn bind(mode: Arc<str>, key: Arc<str>, command: Arc<str>) {
            rust_bind(mode, key, command)
        }

        // Context-aware Bind: Bind EditorCommands to specific contexts
        fn bind_ctx(mode: Arc<str>, context: Arc<str>, key: Arc<str>, command: Arc<str>) {
            rust_bind_ctx(mode, context, key, command)
        }

        // Shell Bind: Bind ShellCommands to "global" context
        fn bind_shell(mode: Arc<str>, key: Arc<str>, cmd: Arc<str>, async_exec: bool) {
            rust_bind_shell(mode, key, cmd, async_exec)
        }

        // Context-aware Shell Bind: Bind ShellCommands to specific contexts
        fn bind_shell_ctx(mode: Arc<str>, context: Arc<str>, key: Arc<str>, cmd: Arc<str>, async_exec: bool) {
            rust_bind_shell_ctx(mode, context, key, cmd, async_exec)
        }

        // Capability-Guarded Function: Global, but checks PERMISSIONS
        fn exec(cmd: Arc<str>) -> Arc<str> {
            rust_exec(cmd)
        }

        fn log(msg: Arc<str>) {
            println!("Editor: {}", msg);
        }
    };

    let mut runtime = match Runtime::new().with_context_type::<EditorContext>() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    runtime.add(lib).unwrap();
    
    if let Ok(home) = env::var("HOME") {
        let mut path = PathBuf::from(home);
        
        path.push(".config/korbin/config.roto");
        
        if path.exists() {
            match runtime.compile(path) {
                Ok(mut pkg) => {
                    let mut ctx = EditorContext {};
                    
                    match pkg.get_function::<fn(Val<SetupToken>) -> ()>("setup") {
                        Ok(setup_fn) => {
                            let _ = setup_fn.call(&mut ctx, Val(SetupToken { _dummy_byte: 0 }));
                        }
                        Err(_) => {
                            eprintln!("Script Error: 'config.roto' found but it is missing the mandatory 'fn setup(t: SetupToken)' function.");
                        }
                    }

                    if let Ok(main_fn) = pkg.get_function::<fn() -> ()>("main") {
                        let _ = main_fn.call(&mut ctx);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to compile config.roto: {}", e);
                }
            }
        }
    }

}

