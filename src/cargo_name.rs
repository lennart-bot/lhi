// Name string
static mut NAME: &str = "";

/// Get name (unsafe, but should be safe unless NAME is being modified)
pub fn name() -> &'static str {
    unsafe { NAME }
}

/// Get name string from a Cargo.toml (unsafe, modifies NAME)
pub fn init_name(cargo_toml: &'static str) -> &str {
    // split by "
    let blocks: Vec<&str> = cargo_toml.split('"').collect();

    let mut name_string = None;
    for (i, &block) in blocks.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if blocks[i - 1].replace(' ', "").replace('\n', "") == "name=" {
            name_string = Some(block)
        }
    }

    // modifiy NAME and return
    unsafe {
        NAME = name_string.unwrap_or("lhi");
    }
    name()
}
