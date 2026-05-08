module 0x1::post_blocks {
    spec main {} proof {
        post assert result == x;
        post {
            let v = x; assert result == v;
        }
    }
}
