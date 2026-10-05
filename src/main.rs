use mlua_bindgen::bindgen::BindgenTransformer;

fn run(args: Vec<String>) -> anyhow::Result<()> {
    if args.len() == 1 {
        let help = [
            "A minimal program that generates luau bindings from rust code.",
            "Arguments:",
            "  - source directory\n  - output file",
            "",
            "Example:",
            "   mlua_bindgen ./src bindings.d.luau"
        ];

        for line in help {
            println!("{line}");
        }

        return Ok(());
    } else if args.len() < 3 {
        return Err(anyhow::anyhow!("Not enough arguments, expected (source directory) (output file)"));
    }

    let dir = &args[1];
    let out = &args[2];
    
    println!("Reading files...");
    let lua_src = BindgenTransformer::new()
        .add_input_dir(dir)
        .parse()
        .map_err(|_| anyhow::anyhow!("Failed to parse rust source directory at {dir}"))?
        .transform_to_lua()
        .map_err(|_| anyhow::anyhow!("Failed to generate luau bindings for source directory at {dir}"))?
        .to_string();

    std::fs::write(out, &lua_src)
        .map_err(|_| anyhow::anyhow!("Failed to write luau declaration file at path {out}"))?;

    println!("Bindings successfully generated.");

    Ok(())
}

fn main() {
    let result = run(std::env::args().collect());

    if let Err(err) = result {
        eprintln!("Encountered an error:\n     {err}");
    }
}