use std::fs::File;
use std::io::{Seek, SeekFrom, Read};
use byteorder::{LittleEndian, ReadBytesExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "porsche992cup_sebring international 2025-11-26 22-40-11.ibt";
    let mut f = File::open(file_path)?;

    // Seek to num_vars at offset 24
    f.seek(SeekFrom::Start(24))?;
    let num_vars = f.read_i32::<LittleEndian>()? as usize;
    let var_header_offset = f.read_i32::<LittleEndian>()? as u64;

    println!("Number of variables: {}", num_vars);
    println!("{:<4} | {:<24} | {:<16} | Description", "Idx", "Channel Name", "Units");
    println!("{}", "-".repeat(120));

    f.seek(SeekFrom::Start(var_header_offset))?;
    for i in 0..num_vars {
        let mut var_buf = vec![0u8; 144];
        f.read_exact(&mut var_buf)?;

        // Extract null-terminated strings using exact official iRacing SDK offsets!
        let name_bytes = &var_buf[16..48];
        let name = String::from_utf8_lossy(name_bytes).trim_end_matches('\0').to_string();

        let desc_bytes = &var_buf[48..112];
        let desc = String::from_utf8_lossy(desc_bytes).trim_end_matches('\0').to_string();

        let unit_bytes = &var_buf[112..144];
        let unit = String::from_utf8_lossy(unit_bytes).trim_end_matches('\0').to_string();

        println!("{:03}  | {:<24} | {:<16} | {}", i, name, unit, desc);
    }

    Ok(())
}
