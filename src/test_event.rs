fn main() {
    let file = std::fs::File::open("").unwrap();
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).unwrap();
    let _t = exif.thumbnail;
}
