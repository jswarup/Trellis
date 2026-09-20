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
    Ok( ())
}
