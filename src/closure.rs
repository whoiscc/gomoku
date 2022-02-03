use crate::interpreter::OperateContext;
use crate::objects::{Closure, ClosureMeta, List};
use std::sync::Arc;

// arguments: 1 ClosureMeta + n_capture varibles
// result: 1 Closure
pub fn operate_new_closure(context: &mut dyn OperateContext) {
    let meta = context.get_argument(0);
    let meta: &ClosureMeta = context.inspect(meta).as_ref().downcast_ref().unwrap();
    let capture_list = (1..=meta.n_capture)
        .map(|i| context.get_argument(i))
        .collect();
    let closure = Closure {
        dispatch: meta.dispatch.clone(),
        stage_list: meta.stage_list.clone(),
        capture_list,
        n_argument: meta.n_argument,
    };
    let closure = context.allocate(Arc::new(closure));
    context.push_result(closure);
}

// arguments: 1 Closure + n_argument varibles
// result: 1 pack of variables (captured + arguments) + 1 Dispatch
pub fn operate_prepare_closure(context: &mut dyn OperateContext) {
    let closure = context.get_argument(0);
    let closure: &Closure = context.inspect(closure).as_ref().downcast_ref().unwrap();
    let mut variable_list = closure.capture_list.clone();
    variable_list.extend((1..=closure.n_argument).map(|i| context.get_argument(i)));
    let dispatch = closure.dispatch.clone();
    let pack = List(variable_list);
    let pack = context.allocate(Arc::new(pack));
    context.push_result(pack);
    let dispatch = context.allocate(Arc::new(dispatch));
    context.push_result(dispatch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{ByteCode, Interpreter, Module, OperateContext};
    use crate::objects::{Dispatch, LeafObject};
    use lazy_static::lazy_static;

    lazy_static! {
        static ref MAIN_MODULE: String = String::from("main");
        static ref START_SYMBOL: String = String::from("start");
        static ref START_DISPATCH: Dispatch = Dispatch {
            module_id: MAIN_MODULE.clone(),
            symbol: START_SYMBOL.clone(),
        };
    }

    #[allow(unused)]
    fn operate_destruct_list(context: &mut dyn OperateContext) {
        let list = context.get_argument(0);
        let list: &List = context.inspect(list).as_ref().downcast_ref().unwrap();
        if let [x, xs @ ..] = &list.0[..] {
            let x = *x;
            let xs = List(xs.to_vec());
            let xs = context.allocate(Arc::new(xs));
            context.push_result(x);
            context.push_result(xs);
        } else {
            panic!("destruct on empty list");
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct I32(i32);
    impl LeafObject for I32 {}

    #[test]
    fn add_two_closure() {
        let mut interp = Interpreter::new();
        let closure_symbol = || String::from("[closure]");
        let assert_i32 = |i| {
            ByteCode::Operate(
                1,
                Box::new(move |context| {
                    assert_eq!(
                        context
                            .inspect(context.get_argument(0))
                            .as_ref()
                            .downcast_ref(),
                        Some(&I32(i))
                    );
                }),
            )
        };
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            symbol_table: [(START_SYMBOL.clone(), 0), (closure_symbol(), 14)]
                .into_iter()
                .collect(),
            program: vec![
                ByteCode::AllocateLiteral(Box::new(move || {
                    Box::new(ClosureMeta {
                        dispatch: Dispatch {
                            module_id: MAIN_MODULE.clone(),
                            symbol: closure_symbol(),
                        },
                        stage_list: Vec::new(),
                        n_capture: 1,
                        n_argument: 1,
                    })
                })),
                ByteCode::AllocateLiteral(Box::new(|| Box::new(I32(2)))),
                // [add two]
                ByteCode::Operate(2, Box::new(operate_new_closure)),
                // 1 [add two]
                ByteCode::AllocateLiteral(Box::new(|| Box::new(I32(1)))),
                // [dispatch] [variable pack] 1 [add two]
                ByteCode::Operate(2, Box::new(operate_prepare_closure)),
                // [add two](1) 1 [add two]
                ByteCode::Call(1),
                ByteCode::AssertFloating(1),
                assert_i32(3),
                // [add two]
                ByteCode::Copy(3),
                ByteCode::AllocateLiteral(Box::new(|| Box::new(I32(40)))),
                ByteCode::Operate(2, Box::new(operate_prepare_closure)),
                ByteCode::Call(1),
                assert_i32(42),
                ByteCode::Return(0),
                // [closure]
                ByteCode::AssertFloating(1),
                ByteCode::Operate(
                    1,
                    Box::new(|context| {
                        let pack = context.get_argument(0);
                        let pack: &List = context.inspect(pack).as_ref().downcast_ref().unwrap();
                        let pack = &pack.0;
                        assert_eq!(pack.len(), 2);
                        let captured: &I32 =
                            context.inspect(pack[0]).as_ref().downcast_ref().unwrap();
                        let argument: &I32 =
                            context.inspect(pack[1]).as_ref().downcast_ref().unwrap();
                        let result = I32(captured.0 + argument.0);
                        let result = context.allocate(Arc::new(result));
                        context.push_result(result);
                    }),
                ),
                ByteCode::Return(1),
            ],
        });
        interp.push_call(START_DISPATCH.clone());
        while interp.has_step() {
            interp.step();
        }
    }
}
