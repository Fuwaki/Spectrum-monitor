use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Stream};
use tokio::sync::mpsc;


pub struct Audio{
    stream:Stream,
    rx:mpsc::Receiver<Vec<f32>>,
    tx:mpsc::Sender<Vec<f32>>
}

impl Audio{
    fn create_stream(tx:mpsc::Sender<Vec<f32>>)->Result<Stream, anyhow::Error>{
        let host = cpal::default_host();
        let device = host.default_input_device().expect("找不到默认输入设备");
        let config = device.default_input_config()?;
        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };
        let stream = device.build_input_stream(
            &config.into(),
            move |d, _: &_|tx.try_send(d.to_vec()).unwrap(),
            err_fn,
            None,
        )?;
        stream.play()?;
        Ok(stream)
    }
    pub fn new()->Self{
        let (tx,rx)=mpsc::channel::<Vec<f32>>(1024);
        let s=Self::create_stream(tx.clone()).unwrap();
        Self{
            stream:s,
            rx,
            tx
        }
    }
    pub fn get_iter(&mut self)->AudioStream<1024>{
        AudioStream{
            rx:&mut self.rx,
            temp:Vec::new()
        }
    }

    pub fn stop(&mut self)->(){
        self.stream.pause().unwrap();
        
    }
}
pub struct AudioStream<'a,const PACK_SIZE:usize>{
    rx:&'a mut mpsc::Receiver<Vec<f32>>,
    temp:Vec<f32>
}
impl <'a,const PACK_SIZE:usize> tokio_stream::Stream for AudioStream<'a,PACK_SIZE>{
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>)
        -> std::task::Poll<Option<Self::Item>>
    {
        if self.temp.len()>=PACK_SIZE{
            let a:Vec<f32>=self.temp.drain(0..PACK_SIZE).collect();
            std::task::Poll::Ready(Some(a.try_into().unwrap()))
        }else{
            match self.rx.poll_recv(cx) {
                std::task::Poll::Ready(Some(data)) => {
                    self.temp.extend(data);
                    self.poll_next(cx)      //这么递归调用不知道可不可以
                }
                std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }
    type Item=[f32;PACK_SIZE];
}
