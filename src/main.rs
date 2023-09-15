use futures_util::{SinkExt, StreamExt, FutureExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tokio::time::{sleep, Duration};
use rand::Rng;
use std::str;

type Grid = [[u8; 50]; 50];

struct State {
    speed: u8,
    pause: bool,
    grid: Grid,
}

const ALIVE_SPAWN_CHANCE: u8 = 10;

#[tokio::main]
async fn main() {

    let listener = TcpListener::bind("127.0.0.1:7936").await.expect("Failed to bind");

    while let Ok((stream, _)) = listener.accept().await {

        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let mut state = State {
        speed: 1,
        grid: spawn_grid(),
        pause: false,
    };

    loop {
        send_grid(&state.grid, &mut ws_stream).await;
        
        if let Some(next) = ws_stream.next().now_or_never() {

            if let Some(message) = next {
                match message {
                    Ok(msg) => {
                        handle_input(msg, &mut state);
                    },
                    Err(err) => {
                        println!("Unable to handle input: {}", err);
                    }
                }   
            }
        }

        sleep(Duration::from_millis(1000 / state.speed as u64)).await;

        if state.pause {
            continue;
        }

        state.grid = next_tick(&state.grid);


    }
}

fn handle_input(input: Message, state: &mut State) {

    let (command, args) = match parse_command(input) {
        Some(command) => command,
        None => {
            println!("Unable to parse command");
            return;
        }
    };    

    match command.as_str() {
        "reset" => {
            (*state).grid = spawn_grid();
        },
        "speed" => {
            // todo: sppeed becomes 0 somehow
            //
            let speed = &mut(*state).speed;

            if let Some(args) = args {

                if args[0].as_str() == "-" {
                    *speed += 1;
                } else if *speed >= 1 {
                    *speed -= 1;
                }
            } else {
                *speed = 1;
            }
        }
        "pause" => {
            (*state).pause = true;
        },
        "play" => {
            (*state).pause = false;
        }
        _ => {
            println!("Unknown command {}", command);
        }
    }
}

fn parse_command(input: Message) -> Option<(String, Option<Vec<String>>)> {

    if !input.is_text() {
        return None;
    }

    let message = input.into_text().unwrap();
    let nested: Vec<&str> = message.split("{").collect();

    if nested.len() < 3 {
        return None;
    }

    for header_line in nested[2].split(",") {

        //println!("header_line: {}", header_line);


        let (name, value) = match header_line.split_once(":") {
            Some((name, value)) => (name.trim_matches('"'), value.trim_matches('"')),
            _ => { continue; }
        };

        if name == "HX-Trigger-Name" && !value.is_empty() {
            
            match value.split_once(":") {
                None => {
                    return Some((value.to_owned(), None))
                },
                Some((command, args)) => {

                    let arguments = args.split(",").map(|a| a.to_owned()).collect();

                    return Some((command.to_owned(), Some(arguments)));
                }
            }
        }    
    }

    None 
}

async fn send_grid(grid: &Grid, ws_stream: &mut WebSocketStream<TcpStream>) {

    ws_stream.send(Message::text(as_html(&grid)))
        .await
        .expect("Error sending grid");
}

fn next_tick(prev: &Grid) -> Grid {
    let mut next = prev.clone();

    for (i, _) in prev.iter().enumerate() {
        for (y, _) in prev[i].iter().enumerate() {

            let an = alive_neighbours(prev, i, y);

            if prev[i][y] == 1 {

                if an < 2 || an > 3 {
                    next[i][y] = 0;
                }
            } else if an == 3 {
                
                next[i][y] = 1;
            }
        }
    }

    next
}

fn alive_neighbours(grid: &Grid, i: usize, y: usize) -> u8 {

    let mut alive = 0;
    let margin_i = grid.len() - 1;
    let margin_y = grid[0].len() - 1;
    let iter: [i8; 3] = [-1, 0, 1];

    for ni in iter {
        for ny in iter {

           if   ni == 0 && ny == 0 ||
                ni == 1 && i == margin_i ||
                ny == 1 && y == margin_y || 
                ni == -1 && i == 0 ||
                ny == -1 && y == 0 {

                    continue;
                }

           let ci = (ni + (i as i8)) as usize;
           let cy = (ny + (y as i8)) as usize;

           if grid[ci][cy] == 1 { alive += 1; }
        }
    }

    alive
}

fn spawn_grid() -> Grid {

    let mut grid: Grid = [[0u8; 50]; 50];

    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
           let num = rand::thread_rng().gen_range(0..100);

           if num > 100 - ALIVE_SPAWN_CHANCE {
                *cell = 1;
           }
        }
    }

    grid

}

fn as_html(grid: &Grid) -> String {
    
    let mut html = String::from("<div id=\"container\" class=\"container\" hx-swap-oob=\"true\">\n");

    for (i, row) in grid.iter().enumerate() {

        html.push_str("<div class=\"row\">\n");

        for (y, cell) in row.iter().enumerate() {
            
            let name = format!("{}:{}", i, y);

            html.push_str(
                format("<label for=\"{}\"></label>", name);
            );
            html.push_str(
                format!("<input type=\"hidden\" id="{}" name=\"{}:{}\" value=\"{}\"", i, y, *cell),
            );
            
            if *cell == 1 {
                html.push_str("\t<span class=\"alive\"></span>\n");
            } else {
                html.push_str("\t<span></span>\n");
            }

        }

        html.push_str("</div>\n");
    }

    html.push_str("</div>");

    html

}
