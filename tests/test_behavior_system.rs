use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::Declaration;

#[test]
fn test_behavior_definition() {
    let code = r#"
        Comparable<T> = {
            compare: (a: T, b: T) i32,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_multiple_behaviors() {
    let code = r#"
        Hashable<T> = {
            hash: (value: T) u64,
        }
        
        Serializable<T> = {
            serialize: (value: T, writer: Ptr<Writer>) Result<void, Error>,
            deserialize: (reader: Ptr<Reader>) Result<T, Error>,
        }
        
        Equatable<T> = {
            equals: (a: T, b: T) bool,
            not_equals: (a: T, b: T) bool,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 3);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_implementation() {
    let code = r#"
        i32_comparable := Comparable<i32>{
            compare: (a: i32, b: i32) i32 {
                a < b ? | true => -1
                        | false => a > b ? | true => 1
                                           | false => 0
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_with_generic_function() {
    let code = r#"
        sort<T> = (items: Ptr<Slice<T>>, cmp: Comparable<T>) void {
            (0..items.value.len()).loop((i) => {
                ((i+1)..items.value.len()).loop((j) => {
                    cmp.compare(items.value[i], items.value[j]) > 0 ?
                        | true => swap(items, i, j)
                        | false => {}
                })
            })
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_usage() {
    let code = r#"
        main = () void {
            numbers := [5, i32]{ 3, 1, 4, 1, 5 }
            sort(Ptr::new(numbers.to_slice()), i32_comparable)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_derivation() {
    let code = r#"
        #derive(Comparable, Hashable)
        Point = {
            x: f64,
            y: f64,
        }
        
        use_derived = () void {
            p1 := Point{ x: 1.0, y: 2.0 }
            p2 := Point{ x: 3.0, y: 4.0 }
            result := Point_comparable.compare(p1, p2)
            hash := Point_hashable.hash(p1)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}

#[test]
fn test_complex_behavior() {
    let code = r#"
        Iterator<T> = {
            next: (self: Ptr<Iterator<T>>) Option<T>,
            has_next: (self: Ptr<Iterator<T>>) bool,
            reset: (self: Ptr<Iterator<T>>) void,
        }
        
        Collection<T> = {
            size: (self: Ptr<Collection<T>>) usize,
            is_empty: (self: Ptr<Collection<T>>) bool,
            contains: (self: Ptr<Collection<T>>, item: T) bool,
            iterator: (self: Ptr<Collection<T>>) Iterator<T>,
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_composition() {
    let code = r#"
        Displayable<T> = {
            display: (value: T) string,
        }
        
        Debug<T> = {
            debug: (value: T) string,
            debug_verbose: (value: T, level: u32) string,
        }
        
        // Combine behaviors
        Printable<T> = {
            display: Displayable<T>.display,
            debug: Debug<T>.debug,
            print: (value: T) void {
                io.print(display(value))
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 3);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_with_constraints() {
    let code = r#"
        find_max<T> = (items: Slice<T>, cmp: Comparable<T>) Option<T> {
            items.len() == 0 ? 
                | true => .None
                | false => {
                    max ::= items[0]
                    (1..items.len()).loop((i) => {
                        cmp.compare(items[i], max) > 0 ?
                            | true => { max = items[i] }
                            | false => {}
                    })
                    .Some(max)
                }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_with_associated_types() {
    let code = r#"
        Functor<F> = {
            map: <A, B>(fa: F<A>, f: (A) B) F<B>,
        }
        
        Monad<M> = {
            pure: <A>(value: A) M<A>,
            bind: <A, B>(ma: M<A>, f: (A) M<B>) M<B>,
            map: <A, B>(ma: M<A>, f: (A) B) M<B> {
                bind(ma, (a: A) M<B> { pure(f(a)) })
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_for_custom_types() {
    let code = r#"
        Person = {
            name: string,
            age: u32,
        }
        
        person_hashable := Hashable<Person>{
            hash: (p: Person) u64 {
                hash_combine(hash_string(p.name), hash_u32(p.age))
            }
        }
        
        person_comparable := Comparable<Person>{
            compare: (a: Person, b: Person) i32 {
                name_cmp := string_comparable.compare(a.name, b.name)
                name_cmp != 0 ?
                    | true => name_cmp
                    | false => u32_comparable.compare(a.age, b.age)
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 3);
}

#[test]
#[ignore] // TODO: Parser needs to be updated to support behavior syntax per Language Spec v1.1.0
fn test_behavior_as_parameter() {
    let code = r#"
        binary_search<T> = (
            items: Slice<T>, 
            target: T, 
            cmp: Comparable<T>
        ) Option<usize> {
            left ::= 0
            right ::= items.len() - 1
            
            loop (left <= right) {
                mid := left + (right - left) / 2
                comparison := cmp.compare(items[mid], target)
                
                comparison ? 
                    | 0 => return .Some(mid)
                    | v -> v < 0 => { left = mid + 1 }
                    | _ => { right = mid - 1 }
            }
            
            .None
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}