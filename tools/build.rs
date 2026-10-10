use spirv_builder::{ ModuleResult, SpirvBuilder };

fn main() -> Result< (), Box< dyn std::error::Error>>
{
    println!( "cargo:rerun-if-changed=src/drove");
    let   artifact = SpirvBuilder::new( "src/drove", "spirv-unknown-vulkan1.1").build()?;
    let   module_path = match artifact.module {
        ModuleResult::SingleModule( path) => path,
        ModuleResult::MultiModule( modules) => {
            modules.iter().for_each( |(entry_point, path)| {
                println!( "cargo:warning=SPIR-V entry point {} => {}", entry_point, path.display());
            });
            return Err( "Drove produced multiple SPIR-V modules; add explicit entry-point mapping".into());
        }
    };
    println!( "cargo:rustc-env=DROVE_SPV_PATH={}", module_path.display());
    copy_python_dlls();
    Ok( ())
}

fn copy_python_dlls()
{
    if std::env::var_os( "CARGO_FEATURE_PYTHON").is_none() {
        return;
    }
    if std::env::var( "CARGO_CFG_TARGET_OS").as_deref() != Ok( "windows") {
        return;
    }
    let out_dir = match std::env::var( "OUT_DIR") {
        Ok( dir) => std::path::PathBuf::from( dir),
        Err( _) => return,
    };
    let target_profile_dir = match out_dir.ancestors().nth( 3) {
        Some( dir) => dir.to_path_buf(),
        None => return,
    };
    let target_deps_dir = target_profile_dir.join( "deps");
    let _ = std::fs::create_dir_all( &target_profile_dir);
    let _ = std::fs::create_dir_all( &target_deps_dir);

    let python_exe = std::env::var( "PYO3_PYTHON").unwrap_or_else( |_| "python".to_string());
    let output = match std::process::Command::new( &python_exe)
        .args( ["-c", "import sys; print(sys.prefix)"])
        .output()
    {
        Ok( out) if out.status.success() => out,
        _ => return,
    };
    let prefix = String::from_utf8_lossy( &output.stdout).trim().to_string();
    if prefix.is_empty() {
        return;
    }
    let prefix_path = std::path::Path::new( &prefix);

    if let Ok( entries) = std::fs::read_dir( prefix_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some( file_name) = path.file_name().and_then( |n| n.to_str()) {
                let lower = file_name.to_lowercase();
                if lower.starts_with( "python") && lower.ends_with( ".dll") {
                    let dest1 = target_profile_dir.join( file_name);
                    let dest2 = target_deps_dir.join( file_name);
                    let _ = std::fs::copy( &path, &dest1);
                    let _ = std::fs::copy( &path, &dest2);
                }
            }
        }
    }
}
