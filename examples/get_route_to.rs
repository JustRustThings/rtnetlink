// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use futures::TryStreamExt;
use rtnetlink::{new_connection, Error, Handle, RouteMessageBuilder};

#[tokio::main]
async fn main() {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    let destinations = [
        "8.8.8.8".parse().unwrap(),
        "127.0.0.8".parse().unwrap(),
        "2001:4860:4860::8888".parse().unwrap(),
        "::1".parse().unwrap(),
    ];
    for dest in destinations {
        println!("getting best route to {}", dest);
        if let Err(e) = dump_route_to(handle.clone(), dest).await {
            eprintln!("{e}");
        }
        println!();
    }
}

async fn dump_route_to(handle: Handle, dest: IpAddr) -> Result<(), Error> {
    let route = match dest {
        IpAddr::V4(v4) => RouteMessageBuilder::<Ipv4Addr>::new()
            .destination_prefix(v4, 32)
            .build(),
        IpAddr::V6(v6) => RouteMessageBuilder::<Ipv6Addr>::new()
            .destination_prefix(v6, 128)
            .build(),
    };
    if let Some(route) = handle.route().get(route).execute().try_next().await? {
        println!("{route:?}");
    }
    Ok(())
}
