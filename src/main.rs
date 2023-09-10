use futures_util::SinkExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tokio::time::{sleep, Duration};
use rand::Rng;

type Grid = [[u8; 50]; 50];

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

    let mut current = spawn_grid();

    send_grid(&current, &mut ws_stream).await;

    loop {
        sleep(Duration::from_millis(1000)).await;

        current = next_tick(&current);
        
        send_grid(&current, &mut ws_stream).await;
    }
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

    for row in grid {

        html.push_str("<div class=\"row\">\n");

        for cell in row {
            
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
