use crate::hd2_pbr::{default_pbr_channel, AdvancedPBR, AoMap, BasicPBR, EmissiveMap, MetallicMap, NormalMap, RoughnessMap};
use clap::Parser;
use colored::Colorize;
use image::{GrayImage, ImageFormat, ImageReader, RgbImage};
use inquire::Confirm;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::exit;

mod hd2_pbr;

const HELP_EXAMPLES: &str =
"EXAMPLES:
    Basic PBR with full-white emissive:
        texswizzle-rs --metallic MetallicMap.png --roughness Roughness.png --ao AOMap.png --basic basic_pbr.png
    Advanced PBR:
        texswizzle-rs --normal NormalMap.png --ao AOMap.png --roughness Roughness.png --advanced advanced_pbr.png
    Advanced PBR with default normal map:
        texswizzle-rs --ao AOMap.png --roughness Roughness.png --advanced advanced_pbr.png
";

#[derive(Parser)]
#[clap(
    group(
        clap::ArgGroup::new("tt")
            .required(true)
            .multiple(false)
    ),
)]
#[command(
    name = "texswizzle",
    after_help = HELP_EXAMPLES,
)]
/// Swizzle textures into composite Helldivers 2 textures.
///
/// Use the --metallic, --normal, --roughness, --ao, and --emissive options
/// to provide textures that will be swizzled. Use --basic or --advanced to
/// produce a basic or advanced pbr respectively. 
/// 
/// Any required texture that is omitted will instead use a sensible default. 
/// For example, omitting the --normal option when creating an --advanced PBR 
/// will behave as if an all-flat normal was provided. All-flat means 
/// \[128, 128, 255] pixel values.
/// 
/// Textures do not have to be the same size. The output will be the same dimensions
/// as the largest input, and the smaller inputs will be upscaled using the 'nearest' 
/// sampling method.
struct Cli {
    #[arg(short, long, value_parser = verify_file)]
    /// path to metallic map texture
    metallic: Option<PathBuf>,
    #[arg(short, long, value_parser = verify_file)]
    /// path to normal map texture
    normal: Option<PathBuf>,
    #[arg(short, long, value_parser = verify_file)]
    /// path to roughness map texture
    roughness: Option<PathBuf>,
    #[arg(long, value_parser = verify_file)]
    /// path to Ambient Occlusion (AO) map texture
    ao: Option<PathBuf>,
    #[arg(short, long, value_parser = verify_file)]
    /// path to emissive map texture
    emissive: Option<PathBuf>,

    #[arg(long, short, group="tt")]
    /// output a PBR for the basic material using metallic, roughness, ao, and emissive textures
    basic: bool,
    #[arg(long, short, group="tt")]
    /// output a PBR for the advanced material using normal, ao, and roughness textures
    advanced: bool,


    #[arg(long, short = 'y')]
    /// overwrite output file if it exists; do not prompt.
    overwrite: bool,

    #[arg(short = 'f', long)]
    /// Use the normal map with an inverted green channel. Does not modify the original file.
    /// Use this if you are passing an OpenGL normal map.
    convert_normal: bool,

    #[arg(long, short)]
    /// list of textures whose types should be inferred by their names.
    /// Textures should generally be passed using the respective options,
    /// but you can use this if you're lazy and confident.
    infer_textures: Option<Vec<PathBuf>>,
    /// output file path. Image type is inferred from the file extension.
    output: PathBuf,
}

pub fn open_as_greyscale(image_path: &Path) -> GrayImage {
    fn open(image_path: &Path) -> Result<GrayImage, Box<dyn Error>> {
        Ok(ImageReader::open(image_path)?.decode()?.into_luma8())
    }

    match open(image_path) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to open {} as greyscale: {e}", image_path.display());
            exit(1);
        }
    }
}

pub fn open_as_rgb(image_path: &Path) -> RgbImage {
    fn open(image_path: &Path) -> Result<RgbImage, Box<dyn Error>> {
        Ok(ImageReader::open(image_path)?.decode()?.into_rgb8())
    }

    match open(image_path) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to open {} as rgb: {e}", image_path.display());
            exit(1);
        }
    }
}

#[derive(Default)]
struct AvailableChannels {
    metallic: Option<MetallicMap>,
    normal: Option<NormalMap>,
    roughness: Option<RoughnessMap>,
    ao: Option<AoMap>,
    emissive: Option<EmissiveMap>,
}

impl From<&Cli> for AvailableChannels {
    fn from(cli: &Cli) -> Self {
        let mut channels = AvailableChannels::default();

        if let Some(channel_p) = &cli.metallic {
            channels.metallic.replace(
                MetallicMap(open_as_greyscale(channel_p))
            );
        }
        if let Some(channel_p) = &cli.normal {
            channels.normal.replace(
                NormalMap(open_as_rgb(channel_p))
            );
        }
        if let Some(channel_p) = &cli.roughness {
            channels.roughness.replace(
                RoughnessMap(open_as_greyscale(channel_p))
            );
        }
        if let Some(channel_p) = &cli.ao {
            channels.ao.replace(
                AoMap(open_as_greyscale(channel_p))
            );
        }
        if let Some(channel_p) = &cli.emissive {
            channels.emissive.replace(
                EmissiveMap(open_as_greyscale(channel_p))
            );
        }

        channels
    }
}

/// Custom validator to check if the path exists and is a file
fn verify_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.exists() {
        return Err(format!("path does not exist: '{s}'"));
    }
    if path.is_dir() {
        return Err(format!("path is a directory, not a file: '{s}'"));
    }
    Ok(path)
}

fn assemble_textures(cli: &Cli) -> AvailableChannels {
    let mut channels = AvailableChannels::from(cli);

    fn try_infer_texture(path: &Path, available_channels: &mut AvailableChannels) {
        let file_name_cased = path.file_name().expect("directories would already be rejected").to_string_lossy();
        let file_name = file_name_cased.to_lowercase();
        let file_base_name = file_name.rsplit_once(".").unwrap_or((file_name.as_ref(), "")).0;
        // TOOD: print out these inferences
        if file_base_name.ends_with("roughness") {
            println!("Taking '{file_name}' as Roughness map.");
            available_channels.roughness.replace(
                RoughnessMap(open_as_greyscale(&path))
            );
        }else if file_base_name.ends_with("metallic") {
            println!("Taking '{file_name}' as Metallic map.");
            available_channels.metallic.replace(
                MetallicMap(open_as_greyscale(&path))
            );
        }else if file_base_name.ends_with("ao") {
            println!("Taking '{file_name}' as AO map.");
            available_channels.ao.replace(
                AoMap(open_as_greyscale(&path))
            );
        }else if file_base_name.ends_with("emissive") {
            println!("Taking '{file_name}' as Emissive map.");
            available_channels.emissive.replace(
                EmissiveMap(open_as_greyscale(&path))
            );
        }else if file_base_name.ends_with("normal") {
            println!("Taking '{file_name}' as Normal map.");
            available_channels.normal.replace(
                NormalMap(open_as_rgb(&path))
            );
        }else {
            write_warning(
                &format!(
                    "Cannot infer type of '{file_name}'. Try adding any of the following to the end of its name:\n\troughness\n\tmetallic\n\tao\n\temissive\n\tnormal"
                )
            );
        }
    }

    if let Some(infer_textures) = &cli.infer_textures {
        for p in infer_textures.iter().by_ref() {
            try_infer_texture(p, &mut channels);
        }
    }

    if cli.convert_normal && let Some(normal) = channels.normal {
        println!("Inverting green channel of normal map to convert between OpenGL and DirectX normal maps");
        channels.normal = Some(normal.flipped());
    }

    channels
}

fn write_err(error_string: &str) {
    eprintln!("Error: {}", error_string.bright_red().bold());
}

fn write_warning(warning_string: &str) {
    let warning_string = format!("Warning: {}", warning_string.bright_yellow().bold());
    println!("{}", warning_string.bright_yellow().bold());
}

pub fn main() {
    let cli = Cli::parse();
    let textures = assemble_textures(&cli);

    let image_format = ImageFormat::from_path(&cli.output).unwrap_or_else(|e| {
        let extension = cli.output.extension()
            .map(|ext| ext.to_string_lossy())
            .unwrap_or("Unknown file type".into());
        write_err(&format!("This program does not support writing '{}' files: {e}", extension));
        exit(1);
    });

    if cli.output.exists() && !cli.overwrite {
        let overwrite= Confirm::new("The output file already exists. Overwrite?")
            .prompt().unwrap_or(false);
        if !overwrite {
            println!("Refusing to overwrite. Exiting...");
            exit(0);
        }else{
            println!("Overwriting.");
        }
    }

    if cli.output.is_dir() {
        write_err(&"Output path is a directory.");
        exit(1);
    }

    if let Some(parent_dir) = cli.output.parent() {
        std::fs::create_dir_all(parent_dir).unwrap_or_else(|e| {
            write_err(&format!("Cannot create output directory: '{}': {e}", parent_dir.display()));
            exit(1);
        });
    }

    let output_file = OpenOptions::new().write(true).create(true).open(&cli.output).unwrap_or_else(|e| {
        write_err(&format!("Cannot open output file: '{}': {e}", cli.output.display()));
        exit(1);
    });
    let mut output_writer = BufWriter::new(output_file);

    if cli.basic {
        let metallic = &textures.metallic.unwrap_or_else(|| {
            write_warning(&"Using default all-black for metallic texture.");
            default_pbr_channel(0).into()
        });
        let roughness = &textures.roughness.unwrap_or_else(|| {
            write_warning(&"Using default all-white for roughness texture.");
            default_pbr_channel(255).into()
        });
        let ao = &textures.ao.unwrap_or_else(|| {
            write_warning(&"Using default all-white for AO texture.");
            default_pbr_channel(255).into()
        });
        let emissive = &textures.emissive.unwrap_or_else(|| {
            write_warning(&"Using default all-white for emissive texture.");
            default_pbr_channel(255).into()
        });
        let basic = BasicPBR::new(metallic, roughness, ao, emissive);

        basic.write_to(&mut output_writer, image_format).unwrap_or_else(|e| {
            write_err(&format!("Failed to write output file: '{}': {e}", cli.output.display()));
            exit(1);
        });
    } else if cli.advanced {
        let normal = &textures.normal.unwrap_or_else(|| {
            write_warning("Using default all-flat normal texture.");
            NormalMap::default()
        });
        let ao = &textures.ao.unwrap_or_else(|| {
            write_warning(&"Using default all-white for AO texture.");
            default_pbr_channel(255).into()
        });
        let roughness = &textures.roughness.unwrap_or_else(|| {
            write_warning(&"Using default all-white for roughness texture.");
            default_pbr_channel(255).into()
        });

        let advanced = AdvancedPBR::new(normal, ao, roughness);
        advanced.write_to(&mut output_writer, image_format).unwrap_or_else(|e| {
            write_err(&format!("Failed to write output file: '{}': {e}", cli.output.display()));
            exit(1);
        });
    } else {
        write_err("Do not know which texture to swizzle to!");
        exit(1)
    }
}