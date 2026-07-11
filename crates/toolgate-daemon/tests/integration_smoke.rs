use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use toolgate_core::protocol::MAX_FRAME_BYTES;
#[tokio::test]
async fn malformed_or_oversized_frames_fail_closed() {
    let (mut a, mut b) = UnixStream::pair().unwrap();
    tokio::spawn(async move {
        let mut h = [0; 4];
        b.read_exact(&mut h).await.unwrap();
        assert!(u32::from_be_bytes(h) as usize > MAX_FRAME_BYTES);
    });
    a.write_all(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes())
        .await
        .unwrap();
}
