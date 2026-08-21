use super::common::string_unary_fn;

string_unary_fn!(Capitalize, |input: &str| {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = String::new();
            result.extend(first.to_uppercase());
            result.extend(chars.flat_map(|c| c.to_lowercase()));
            result
        }
    }
});

string_unary_fn!(TitleCase, |input: &str| {
    let mut result = String::new();
    let mut new_word = true;
    for ch in input.chars() {
        if ch == ' ' {
            result.push(ch);
            new_word = true;
        } else if new_word {
            result.extend(ch.to_uppercase());
            new_word = false;
        } else {
            result.extend(ch.to_lowercase());
        }
    }
    result
});

fn split_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in input.chars() {
        if ch == '-' || ch == '_' || ch == ' ' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else if ch.is_uppercase() {
            if !current.is_empty() && current.chars().last().map_or(false, |c| c.is_lowercase()) {
                words.push(std::mem::take(&mut current));
            }
            current.push(ch);
        } else {
            if !current.is_empty()
                && current.chars().last().map_or(false, |c| c.is_uppercase())
                && current.chars().count() > 1
            {
                let last = current.pop().unwrap();
                words.push(std::mem::take(&mut current));
                current.push(last);
            }
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words.into_iter().map(|w| w.to_lowercase()).collect()
}

string_unary_fn!(SnakeCase, |input: &str| split_words(input).join("_"));
string_unary_fn!(KebabCase, |input: &str| split_words(input).join("-"));

string_unary_fn!(CamelCase, |input: &str| {
    let words = split_words(input);
    let mut result = String::new();
    for (i, word) in words.iter().enumerate() {
        if i == 0 {
            result.push_str(word);
        } else {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.extend(first.to_uppercase());
                result.extend(chars);
            }
        }
    }
    result
});

string_unary_fn!(PascalCase, |input: &str| {
    let words = split_words(input);
    let mut result = String::new();
    for word in &words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.extend(chars);
        }
    }
    result
});
