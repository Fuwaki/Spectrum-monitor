use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Stream};
use std::{os::unix::thread, pin::Pin, sync::{mpsc, Arc}, thread::sleep, time::Duration};
use tokio::sync::Notify;

pub async fn run(s: mpsc::Sender<Vec<f32>>, n: Arc<Notify>) -> Result<(), anyhow::Error> {
    let ss: Pin<Box<Stream>>;
    {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("找不到默认输入设备");
        let config = device.default_input_config()?;
        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };
        let stream = device.build_input_stream(
            &config.into(),
            move |d, _: &_| s.send(d.to_vec()).unwrap(),
            err_fn,
            None,
        )?;
        stream.play()?;
        ss=Box::pin(stream);
    }
    // n.notified().await;
    drop(ss);
    Ok(())
}
