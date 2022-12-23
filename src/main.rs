use anyhow::Context;
use image::io::Reader;
use std::{
    env,
    io::{stdin, stdout, Write},
    process::exit,
    thread,
    time::Duration,
};
use termdrawserver::{ClientPayload, Pixel, PixelColour, ServerPayload};
use tungstenite::{connect, Message};
use uuid::Uuid;

#[derive(Debug)]
struct Args {
    server_url: String,
    filename: String,
    scale: usize,
}

fn main() -> Result<(), anyhow::Error> {
    let args = get_args();

    let img = Reader::open(&args.filename)
        .with_context(|| format!("Could not open file {}", args.filename))?
        .decode()
        .with_context(|| format!("Could not decode file {}", args.filename))?
        .to_rgba8();

    let (mut socket, _) = connect(&args.server_url).with_context(|| {
        format!(
            "Could not connect to termdrawserver instance on {}",
            args.server_url
        )
    })?;

    print!("Enter the room id: ");
    stdout().flush().context("Could not flush StdOut")?;
    let mut room_id = String::new();
    stdin()
        .read_line(&mut room_id)
        .context("Could not read from StdIn")?;
    let room_id = Uuid::parse_str(room_id.trim())
        .with_context(|| format!("Invalid v4 UUID {}", room_id.trim()))?;

    socket
        .write_message(Message::Text(
            serde_json::to_string(&ClientPayload::JoinRoom(room_id)).unwrap(),
        ))
        .context("Could not send JoinRoom OPCode")?;

    while let Ok(Message::Text(msg)) = socket.read_message() {
        if let Ok(payload) = serde_json::from_str::<ServerPayload>(&msg) {
            match payload {
                ServerPayload::Join { .. } => {
                    break;
                }
                ServerPayload::RoomNotFound => anyhow::bail!("Unknown room, bailing"),
                _ => {}
            }
        }
    }

    let (mut x, mut y, mut sent) = (0, 0, 0);
    img.rows().step_by(args.scale * 2).for_each(|p| {
        p.step_by(args.scale).for_each(|p| {
            let rgba = p.0;
            let value = rgba[0] as u16 + rgba[1] as u16 + rgba[2] as u16;
            if value > 300 {
                sent += 1;
                print!("0");
                socket
                    .write_message(Message::Text(
                        serde_json::to_string(&ClientPayload::Draw(Pixel {
                            x,
                            y,
                            colour: PixelColour::White,
                        }))
                        .unwrap(),
                    ))
                    .context("Could not send JoinRoom OPCode")
                    .unwrap();
                if sent % 50 == 0 {
                    thread::sleep(Duration::from_millis(100));
                }
            } else {
                print!(" ");
            }
            x += 1;
        });
        println!("");
        y += 1;
        x = 0;
    });

    Ok(())
}

fn get_args() -> Args {
    let mut args = env::args().skip(1);
    let server_url = match args.next() {
        Some(server_url) => server_url,
        None => {
            println!("Usage: termdrawascii <termdrawserver url> <image-path> [zoom-in-scale=1]");
            exit(1)
        }
    };
    let filename = match args.next() {
        Some(filename) => filename,
        None => {
            println!("Usage: termdrawascii <termdrawserver url> <image-path> [zoom-in-scale=1]");
            exit(1)
        }
    };
    let scale: usize = args
        .next()
        .unwrap_or_else(|| "1".to_string())
        .parse()
        .unwrap();

    Args {
        server_url,
        filename,
        scale,
    }
}
