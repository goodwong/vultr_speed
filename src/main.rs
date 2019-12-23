use async_std::sync::{Arc, Mutex};
use bytesize::ByteSize;
use chrono;
use reqwest;
use std::io::{stdout, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use termion;
use tokio;

#[derive(Clone)]
struct PacketReport {
    timestamp: u128,
    size: usize,
}

#[derive(Clone)]
struct Endpoint {
    name: String,
    url: String,
}

// todo 目前存在的问题：
// 1. 并行下载，表现为带宽冲突，需要改为轮候测试
// 2. 需要去除开始的前5秒
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoints = [
        Endpoint {
            name: "Frankfurt".to_string(),
            url: "https://fra-de-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Paris".to_string(),
            url: "https://par-fr-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        //Endpoint {
        //    name: "Amsterdam".to_string(),
        //    url: "https://ams-cl-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        //},
        Endpoint {
            name: "London".to_string(),
            url: "https://lon-gb-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "New York".to_string(),
            url: "https://nj-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Singapore".to_string(),
            url: "https://sgp-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Toronto".to_string(),
            url: "https://tor-ca-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Chicago".to_string(),
            url: "https://il-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Atlanta".to_string(),
            url: "https://ga-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Miami".to_string(),
            url: "https://fl-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Tokyo".to_string(),
            url: "https://hnd-jp-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Dallas".to_string(),
            url: "https://tx-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Seattle".to_string(),
            url: "https://wa-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Silicon Val".to_string(),
            url: "https://sjo-ca-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Los Angeles".to_string(),
            url: "https://lax-ca-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Sydney".to_string(),
            url: "https://syd-au-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
    ];
    let logs = Arc::new(Mutex::from(vec![]));

    let column_width = 12;
    let max_speed = 8_000_000;

    let mut loop_index: i32 = 0;
    loop {
        // 表头
        if loop_index % 10 == 0 {
            println!();
            print!("{:8}  ", "");
            for (_, endpoint) in endpoints.iter().enumerate() {
                print!("{:^width$} ", endpoint.name, width = column_width);
            }
            println!();
        }
        loop_index += 1;

        // 行头
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        print!("{:10}{}", timestamp, termion::cursor::Down(9));
        stdout().flush()?;
        for (_, endpoint) in endpoints.iter().enumerate() {
            // 启动下载
            {
                let receiver = logs.clone();
                let endpoint = endpoint.clone();
                tokio::spawn(async move {
                    download(endpoint, receiver).await.unwrap();
                });
            }

            // 读取10秒数据
            print!("{}", termion::cursor::Right(1));
            print!("{}", termion::cursor::Up(9));
            stdout().flush()?;

            // 去掉开头10秒数据
            tokio::timer::delay_for(Duration::from_millis(10_000)).await;
            for i in 0..10 {
                // 等待1秒
                tokio::timer::delay_for(Duration::from_millis(1000)).await;
                // 收取日志
                let logs: Vec<PacketReport> = logs.lock().await.drain(0..).collect();

                let bytes = logs.iter().fold(0, |a, b| a + b.size);
                let speed = bytes / 1;

                if i > 0 {
                    print!("{}", termion::cursor::Down(1));
                    print!("{}", termion::cursor::Left(column_width as u16));
                }
                print_speed(speed, max_speed, column_width);
                stdout().flush()?;
            }
        }
        println!();
    }
}

async fn download(
    endpoint: Endpoint,
    receiver: Arc<Mutex<Vec<PacketReport>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 去掉开头10秒（含连接时间）
    // 只取中间10秒
    let start = now();

    let mut res = reqwest::get(endpoint.url.as_str()).await?;
    while let Some(chunk) = res.chunk().await? {
        let timestamp = now();
        let size = chunk.len();
        if timestamp - start < 10_000 {
            continue;
        }

        if timestamp - start >= 20_000 {
            break;
        }
        receiver.lock().await.push(PacketReport { timestamp, size });
    }
    Ok(())
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn print_speed(speed: usize, max: usize, width: usize) {
    let output = format!(
        "{:<width$}",
        ByteSize::b(speed as u64).to_string() + "/s",
        width = width
    );
    let speed_bar_len = usize::min(width, width * speed as usize / max);
    let (part1, part2) = if speed >= max {
        (
            format!("\x1B[7;31m{}\x1B[0m", &output[0..speed_bar_len]),
            format!("\x1B[31;m{}\x1B[0m", &output[speed_bar_len..]),
        )
    } else {
        (
            format!("\x1B[7m{}\x1B[0m", &output[0..speed_bar_len]),
            format!("\x1B[0m{}\x1B[0m", &output[speed_bar_len..]),
        )
    };
    print!("{}{}", part1, part2);
}
