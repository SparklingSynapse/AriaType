use ariatype_lib::audio::beep_generator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    beep_generator::generate_beep_files()?;
    println!("Generated start_beep.wav and stop_beep.wav");
    Ok(())
}
