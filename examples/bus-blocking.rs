#![warn(rust_2018_idioms)]

#[cfg(feature = "bus")]
fn main() {
    use systemd::bus;
    let mut bus = bus::Bus::default().unwrap();

    let bn = bus::BusName::from_bytes(b"com.codyps.systemd-test\0").unwrap();
    bus.request_name(bn, 0).unwrap();
    println!("got name {bn:?}");

    let op = bus::ObjectPath::from_bytes(b"/com/codyps/systemd_test\0").unwrap();
    bus.add_object(op, |m| {
        println!("message: {m:?}");
        Ok(())
    })
    .unwrap();
    println!("added object: {op:?}");

    loop {
        println!("wait?");
        bus.wait(None).unwrap();
        match bus.process().unwrap() {
            Some(Some(m)) => println!("handled message: {:?}", m.as_ref()),
            Some(None) => println!("made progress"),
            None => println!("no progress"),
        }
    }
}

#[cfg(not(feature = "bus"))]
fn main() {
    println!("bus disabled");
}
