/// An enum used to distinguish between setters and getters. When parsing these, the only way to distinguish
/// them is to look at their attribute. Functions that parse fields can take this enum to apply custom rules:
/// 
/// For example, a getter can't contain any arguments, while a setter can only contain one.
pub enum FieldKind {
    Getter,
    Setter
}