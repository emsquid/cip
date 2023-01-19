use crate::options::{Action, Options};
use crate::utils::{
    fit_bounds, get_temp_file, move_cursor, restore_cursor, save_cursor, save_in_tmp_file,
};
use base64::{engine::general_purpose, Engine as _};
use std::io::{Error, Write};

const KITTY_PREFIX: &str = "pic-tty-graphics-protocol.";
const PROTOCOL_START: &str = "\x1b_G";
const PROTOCOL_END: &str = "\x1b\\";

fn send_graphics_command(
    stdout: &mut impl Write,
    command: &str,
    payload: Option<&str>,
) -> Result<(), Error> {
    let data = general_purpose::STANDARD.encode(payload.unwrap_or(""));

    stdout.write(format!("{}{};{}{}", PROTOCOL_START, command, data, PROTOCOL_END).as_bytes())?;
    stdout.flush()
}

fn clear(stdout: &mut impl Write) -> Result<(), Error> {
    send_graphics_command(stdout, "a=d,d=a", None)
}

fn load(stdout: &mut impl Write, options: &Options) -> Result<(), Error> {
    let image = image::open(&options.path).unwrap().to_rgba8();
    let (mut tempfile, pathbuf) = get_temp_file(KITTY_PREFIX)?;
    save_in_tmp_file(image.as_raw(), &mut tempfile)?;

    let id = match options.id {
        Some(id) => id,
        None => panic!("Error: Load: id is required"),
    };

    let command = format!(
        "a=t,t=t,f=32,s={},v={},i={},q=2",
        image.width(),
        image.height(),
        id
    );

    send_graphics_command(stdout, &command, pathbuf.to_str())
}

fn display(stdout: &mut impl Write, options: &Options) -> Result<(), Error> {
    let (mut tempfile, pathbuf) = get_temp_file(KITTY_PREFIX)?;
    let (command, payload) = match options.id {
        Some(id) => {
            let size = imagesize::size(&options.path).unwrap();
            let (width, height) = (size.width as u32, size.height as u32);

            let (cols, rows) =
                fit_bounds(width, height, options.cols, options.rows, options.upscale);

            (format!("a=p,c={},r={},i={},q=2", cols, rows, id), None)
        }
        None => {
            let image = image::open(&options.path).unwrap().to_rgba8();
            let (width, height) = image.dimensions();
            save_in_tmp_file(image.as_raw(), &mut tempfile)?;
            drop(tempfile);

            let (cols, rows) =
                fit_bounds(width, height, options.cols, options.rows, options.upscale);

            (
                format!(
                    "a=T,t=t,f=32,s={},v={},c={},r={},q=2",
                    width, height, cols, rows
                ),
                pathbuf.to_str(),
            )
        }
    };

    match (options.x, options.y) {
        (None, None) => send_graphics_command(stdout, &command, payload),
        _ => {
            save_cursor(stdout)?;
            move_cursor(stdout, options.x.unwrap_or(0), options.y.unwrap_or(0))?;
            send_graphics_command(stdout, &command, payload)?;
            restore_cursor(stdout)
        }
    }?;

    stdout.write(b"\n")?;
    stdout.flush()
}

fn load_and_display(stdout: &mut impl Write, options: &Options) -> Result<(), Error> {
    load(stdout, options)?;
    display(stdout, options)
}

pub fn preview(stdout: &mut impl Write, options: &Options) -> Result<(), Error> {
    match options.action {
        Action::Load => load(stdout, options),
        Action::Display => display(stdout, options),
        Action::LoadAndDisplay => load_and_display(stdout, options),
        Action::Clear => clear(stdout),
    }
}