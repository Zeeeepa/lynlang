extern crate test_utils;

use zen::ast::{self, AstType, Expression, Statement, BinaryOperator, ConditionalArm, Pattern};
use test_utils::TestContext;
use inkwell::context::Context;
use test_utils::test_context;

#[test]
// #[ignore = "LLVM physreg copy instruction error - needs investigation"]
fn test_conditional_expression() {
    test_context!(|test_context: &mut TestContext| {
        let program = ast::Program::from_functions(vec![ast::Function { type_params: vec![],  
            name: "main".to_string(),
            args: vec![],
            return_type: AstType::I64,
            body: vec![
                Statement::Return(Expression::Conditional {
                    scrutinee: Box::new(Expression::BinaryOp {
                        left: Box::new(Expression::Integer64(1)),
                        op: BinaryOperator::Equals,
                        right: Box::new(Expression::Integer64(1)),
                    }),
                    arms: vec![
                        ConditionalArm { 
                            pattern: Pattern::Literal(Expression::Boolean(true)), 
                            guard: None, 
                            body: Expression::Integer64(42) 
                        },
                        ConditionalArm { 
                            pattern: Pattern::Literal(Expression::Boolean(false)), 
                            guard: None, 
                            body: Expression::Integer64(0) 
                        },
                    ],
                }),
            ],
        }]);

        test_context.compile(&program).unwrap();
        let result = test_context.run().unwrap();
        assert_eq!(result, 42);
    });
} 