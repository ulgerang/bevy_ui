use crate::ElementNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selector {
    pub(crate) parts: Vec<SelectorPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectorPart {
    pub(crate) combinator: Option<Combinator>,
    simple: SimpleSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SimpleSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
    pseudo: Option<PseudoClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeSelector {
    name: String,
    operator: AttributeOperator,
    value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeOperator {
    Presence,
    Exact,
    Includes,
    DashMatch,
    Prefix,
    Suffix,
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PseudoClass {
    Hover,
    Active,
    Focus,
    Disabled,
    Checked,
    FocusWithin,
    FocusVisible,
}

impl Selector {
    pub(crate) fn parse_group(input: &str) -> Vec<Result<Self, String>> {
        split_selector_group(input)
            .into_iter()
            .map(|member| {
                Self::parse(&member).ok_or_else(|| {
                    if member.trim().is_empty() {
                        "empty selector".to_string()
                    } else {
                        "unsupported selector syntax".to_string()
                    }
                })
            })
            .collect()
    }

    pub(crate) fn parse(input: &str) -> Option<Self> {
        let parts = split_selector(input)
            .into_iter()
            .map(|(combinator, token)| {
                SimpleSelector::parse(&token).map(|simple| SelectorPart { combinator, simple })
            })
            .collect::<Option<Vec<_>>>()?;

        if parts.is_empty() {
            None
        } else {
            Some(Self { parts })
        }
    }

    pub(crate) fn specificity(&self) -> u32 {
        self.parts
            .iter()
            .map(|part| part.simple.specificity())
            .sum()
    }

    pub(crate) fn matches_with_state(
        &self,
        path: &[&ElementNode],
        state: Option<PseudoClass>,
        runtime_state_mode: bool,
    ) -> Option<u32> {
        self.matches_with_state_scope(path, state, runtime_state_mode, StateMatchScope::Terminal)
    }

    pub(crate) fn matches_with_ancestor_state(
        &self,
        path: &[&ElementNode],
        state: PseudoClass,
    ) -> Option<u32> {
        if !self.has_nonterminal_pseudo(state) {
            return None;
        }
        self.matches_with_state_scope(path, Some(state), true, StateMatchScope::NonTerminal)
    }

    fn has_nonterminal_pseudo(&self, state: PseudoClass) -> bool {
        self.parts
            .iter()
            .take(self.parts.len().saturating_sub(1))
            .any(|part| part.simple.pseudo == Some(state))
    }

    pub(crate) fn has_terminal_pseudo(&self, state: PseudoClass) -> bool {
        self.parts
            .last()
            .is_some_and(|part| part.simple.pseudo == Some(state))
    }

    fn matches_with_state_scope(
        &self,
        path: &[&ElementNode],
        state: Option<PseudoClass>,
        runtime_state_mode: bool,
        state_scope: StateMatchScope,
    ) -> Option<u32> {
        if path.is_empty() {
            return None;
        }

        let mut path_index = path.len() - 1;
        let terminal_state = (state_scope == StateMatchScope::Terminal)
            .then_some(state)
            .flatten();
        let mut bonus = self.parts.last()?.simple.matches_with_state(
            path[path_index],
            terminal_state,
            runtime_state_mode,
        )?;

        for part_index in (0..self.parts.len().saturating_sub(1)).rev() {
            match self.parts[part_index + 1]
                .combinator
                .unwrap_or(Combinator::Descendant)
            {
                Combinator::Child => {
                    if path_index == 0 {
                        return None;
                    }
                    path_index -= 1;
                    bonus += self.parts[part_index].simple.matches_with_state(
                        path[path_index],
                        (state_scope == StateMatchScope::NonTerminal)
                            .then_some(state)
                            .flatten(),
                        runtime_state_mode,
                    )?;
                }
                Combinator::Descendant => {
                    let mut matched = None;
                    for ancestor_index in (0..path_index).rev() {
                        if let Some(part_bonus) = self.parts[part_index].simple.matches_with_state(
                            path[ancestor_index],
                            (state_scope == StateMatchScope::NonTerminal)
                                .then_some(state)
                                .flatten(),
                            runtime_state_mode,
                        ) {
                            matched = Some((ancestor_index, part_bonus));
                            break;
                        }
                    }
                    let (ancestor_index, part_bonus) = matched?;
                    path_index = ancestor_index;
                    bonus += part_bonus;
                }
            }
        }

        Some(bonus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateMatchScope {
    Terminal,
    NonTerminal,
}

impl SimpleSelector {
    fn parse(input: &str) -> Option<Self> {
        let mut selector = Self::default();
        let chars = input.trim().chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return None;
        }

        let mut index = 0;
        if is_ident_start(chars[index]) || chars[index] == '*' {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            if chars[start] != '*' {
                selector.tag = Some(
                    chars[start..index]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase(),
                );
            }
        }

        while index < chars.len() {
            match chars[index] {
                '#' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.id = Some(value);
                    index = next;
                }
                '.' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.classes.push(value);
                    index = next;
                }
                '[' => {
                    let (attribute, next) = parse_attribute_selector(&chars, index)?;
                    selector.attributes.push(attribute);
                    index = next;
                }
                ':' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.pseudo = PseudoClass::parse(&value);
                    selector.pseudo?;
                    index = next;
                }
                _ => return None,
            }
        }

        Some(selector)
    }

    fn specificity(&self) -> u32 {
        let mut score = 0;
        if self.id.is_some() {
            score += 100;
        }
        score += (self.classes.len() + self.attributes.len()) as u32 * 10;
        if self.pseudo.is_some() {
            score += 10;
        }
        if self.tag.is_some() {
            score += 1;
        }
        score
    }

    fn matches_with_state(
        &self,
        node: &ElementNode,
        state: Option<PseudoClass>,
        runtime_state_mode: bool,
    ) -> Option<u32> {
        let mut bonus = 0;

        if let Some(tag) = &self.tag {
            if tag == &node.tag {
                if node.widget_type() != node.tag {
                    bonus += 1;
                }
            } else if tag != node.widget_type() {
                return None;
            }
        }

        if let Some(id) = &self.id {
            if node.id.as_deref() != Some(id.as_str()) {
                return None;
            }
        }

        for class_name in &self.classes {
            if !node.classes.iter().any(|candidate| candidate == class_name) {
                return None;
            }
        }

        for attr in &self.attributes {
            let value = node.attr(&attr.name)?;
            if !attr.matches(value) {
                return None;
            }
        }

        if let Some(pseudo) = self.pseudo {
            if !pseudo.matches(node, state, runtime_state_mode) {
                return None;
            }
        }

        Some(bonus)
    }
}

impl PseudoClass {
    fn parse(input: &str) -> Option<Self> {
        match input {
            "hover" => Some(Self::Hover),
            "active" => Some(Self::Active),
            "focus" => Some(Self::Focus),
            "disabled" => Some(Self::Disabled),
            "checked" => Some(Self::Checked),
            "focus-within" => Some(Self::FocusWithin),
            "focus-visible" => Some(Self::FocusVisible),
            _ => None,
        }
    }

    fn matches(
        self,
        node: &ElementNode,
        state: Option<PseudoClass>,
        runtime_state_mode: bool,
    ) -> bool {
        if state == Some(self) {
            return true;
        }
        match self {
            Self::Disabled if !runtime_state_mode => node.attr("disabled").is_some(),
            Self::Checked if !runtime_state_mode => node.attr("checked").is_some(),
            Self::Disabled
            | Self::Hover
            | Self::Active
            | Self::Focus
            | Self::Checked
            | Self::FocusWithin
            | Self::FocusVisible => false,
        }
    }
}

impl AttributeSelector {
    fn matches(&self, value: &str) -> bool {
        match (self.operator, self.value.as_deref()) {
            (AttributeOperator::Presence, _) => true,
            (_, None) => true,
            (AttributeOperator::Exact, Some(expected)) => value == expected,
            (AttributeOperator::Includes, Some(expected)) => {
                !expected.is_empty() && value.split_whitespace().any(|token| token == expected)
            }
            (AttributeOperator::DashMatch, Some(expected)) => {
                value == expected
                    || value
                        .strip_prefix(expected)
                        .is_some_and(|rest| rest.starts_with('-'))
            }
            (AttributeOperator::Prefix, Some(expected)) => value.starts_with(expected),
            (AttributeOperator::Suffix, Some(expected)) => value.ends_with(expected),
            (AttributeOperator::Substring, Some(expected)) => {
                !expected.is_empty() && value.contains(expected)
            }
        }
    }
}

fn split_selector_group(input: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut quote = None;

    for ch in input.chars() {
        match ch {
            '"' | '\'' if bracket_depth > 0 => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                current.push(ch);
            }
            '[' if quote.is_none() => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' if quote.is_none() => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if bracket_depth == 0 && quote.is_none() => {
                members.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    members.push(current.trim().to_string());
    members
}

fn split_selector(input: &str) -> Vec<(Option<Combinator>, String)> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut pending = None;

    for ch in input.trim().chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            '>' if depth == 0 => {
                push_selector_part(&mut parts, &mut current, pending.take());
                pending = Some(Combinator::Child);
            }
            ch if ch.is_whitespace() && depth == 0 => {
                if current.trim().is_empty() {
                    if !parts.is_empty() && pending.is_none() {
                        pending = Some(Combinator::Descendant);
                    }
                } else {
                    push_selector_part(&mut parts, &mut current, pending.take());
                    if !parts.is_empty() && pending.is_none() {
                        pending = Some(Combinator::Descendant);
                    }
                }
            }
            _ => current.push(ch),
        }
    }

    push_selector_part(&mut parts, &mut current, pending);
    if let Some(first) = parts.first_mut() {
        first.0 = None;
    }
    parts
}

fn push_selector_part(
    parts: &mut Vec<(Option<Combinator>, String)>,
    current: &mut String,
    combinator: Option<Combinator>,
) {
    let token = current.trim();
    if !token.is_empty() {
        parts.push((combinator, token.to_string()));
    }
    current.clear();
}

fn parse_ident(chars: &[char], start: usize) -> Option<(String, usize)> {
    if start >= chars.len() || !is_ident_start(chars[start]) {
        return None;
    }

    let mut index = start + 1;
    while index < chars.len() && is_ident_continue(chars[index]) {
        index += 1;
    }

    Some((chars[start..index].iter().collect(), index))
}

fn parse_attribute_selector(chars: &[char], start: usize) -> Option<(AttributeSelector, usize)> {
    let mut index = start + 1;
    let mut raw = String::new();
    while index < chars.len() && chars[index] != ']' {
        raw.push(chars[index]);
        index += 1;
    }
    if index >= chars.len() {
        return None;
    }

    let (name, operator, value) = parse_attribute_body(&raw);

    if name.is_empty() {
        return None;
    }

    Some((
        AttributeSelector {
            name,
            operator,
            value,
        },
        index + 1,
    ))
}

fn parse_attribute_body(raw: &str) -> (String, AttributeOperator, Option<String>) {
    for (token, operator) in [
        ("~=", AttributeOperator::Includes),
        ("|=", AttributeOperator::DashMatch),
        ("^=", AttributeOperator::Prefix),
        ("$=", AttributeOperator::Suffix),
        ("*=", AttributeOperator::Substring),
        ("=", AttributeOperator::Exact),
    ] {
        if let Some((name, value)) = raw.split_once(token) {
            return (
                name.trim().to_string(),
                operator,
                Some(unquote_attribute_value(value.trim())),
            );
        }
    }

    (raw.trim().to_string(), AttributeOperator::Presence, None)
}

fn unquote_attribute_value(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let Some(last) = value.chars().last() else {
        return String::new();
    };
    if (first == '"' || first == '\'') && last == first && value.len() >= 2 {
        value[1..value.len() - first.len_utf8()].to_string()
    } else {
        value.to_string()
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '-'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
