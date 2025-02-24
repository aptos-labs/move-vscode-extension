module 0x1::loop_labels {
    fun main() {
        'label: loop {
            break 'label;
            continue 'label;
        };
        'label: while (true) {
            break 'label;
            continue 'label;
        }
    }
}
