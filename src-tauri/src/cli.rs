use poster_maker_lib::{default_output_name, generate_poster_file};
use poster_maker_lib::layout::default_options;
use std::path::Path;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or_else(usage)?;
    let mut output = None;
    let mut grid = (3, 2);
    let mut overwrite = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => output = Some(args.next().ok_or_else(usage)?),
            "--grid" => grid = parse_grid(&args.next().ok_or_else(usage)?)?,
            "--overwrite" | "-f" => overwrite = true,
            "-h" | "--help" => return Err(usage()),
            _ => return Err(format!("Unknown argument: {arg}\n{}", usage())),
        }
    }

    let output_name = output.unwrap_or_else(|| default_output_name(Path::new(&input)));
    let options = default_options(grid.0, grid.1);
    let result = generate_poster_file(input, output_name, overwrite, options).map_err(|e| e.to_string())?;
    println!("Generated {} pages: {}", result.pages, result.output);
    Ok(())
}

fn parse_grid(value: &str) -> Result<(u32, u32), String> {
    let normalized = value.trim().replace('×', "x").to_lowercase();
    let (cols, rows) = normalized.split_once('x').ok_or_else(|| "Grid must look like 3x2".to_string())?;
    let cols = cols.parse::<u32>().map_err(|_| "Invalid grid columns".to_string())?;
    let rows = rows.parse::<u32>().map_err(|_| "Invalid grid rows".to_string())?;
    if cols == 0 || rows == 0 || cols > 12 || rows > 12 {
        return Err("Grid range must be 1..12".to_string());
    }
    Ok((cols, rows))
}

fn usage() -> String {
    "Usage: poster-maker-cli <image> [--grid 3x2] [-o output.pdf] [--overwrite]".to_string()
}
