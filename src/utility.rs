use std::any::type_name;

pub fn short_type_name<T>() -> &'static str {
    let name = type_name::<T>();
    let Some(name) = name.split('<').next() else {
        return name;
    };
    let Some(name) = name.split(':').last() else {
        return name;
    };
    name
}
