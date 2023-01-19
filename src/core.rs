#[derive(Debug)]
pub enum Command {
  // String command
  Get(Key),
  Set(Key, Vec<u8>)
}

#[derive(Debug)]
pub struct Key(pub Vec<u8>);