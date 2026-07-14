#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Action {
    MoveForward,
    Movebackwards,
    MoveRight,
    MoveLeft,
    //Jump,
    //Sprint,
    MoveUp,   //this is only for creative mode camera,
    MoveDown, //this is only for creative mode camrea,
}
