use crate::interpreter::OperateContext;
use crate::objects::{Closure, List};

impl Closure {
    // arguments: 1 Closure
    // result: 1 pack of variables (captured) + 1 Dispatch
    pub fn operate_apply(context: &mut dyn OperateContext) {
        let closure = context.inspect(context.get_argument(0));
        let closure: &Closure = (*closure).as_ref().downcast_ref().unwrap();
        let dispatch = closure.dispatch.clone();
        let pack = List(closure.capture_list.clone());
        let pack = context.allocate(pack.into());
        context.push_result(pack);
        let dispatch = context.allocate(dispatch.into());
        context.push_result(dispatch);
    }

    // arguments: 1 mutable Closure + 1 pack of variables
    // no result, closure capture list updated
    pub fn operate_capture(context: &mut dyn OperateContext) {
        return;
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{ByteCode, Interpreter, Module, ModuleId, OperateContext};
    use crate::objects::{Dispatch, LeafObject};
    use crate::GeneralInterface;
    use std::sync::Mutex;

    fn main_module() -> ModuleId {
        String::from("main")
    }
    fn start_symbol() -> String {
        String::from("start")
    }
    fn start_dispatch() -> Dispatch {
        Dispatch {
            module_id: main_module(),
            symbol: start_symbol(),
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct I32(i32);
    impl LeafObject for I32 {}
    impl I32 {
        fn operate_add_two(context: &mut dyn OperateContext) {
            let int_a = context.get_argument(0);
            let int_a: I32 = *context.inspect(int_a).as_ref().downcast_ref().unwrap();
            let int_b = context.get_argument(1);
            let int_b: I32 = *context.inspect(int_b).as_ref().downcast_ref().unwrap();
            let int_c = context.allocate(Arc::new(I32(int_a.0 + int_b.0)));
            context.push_result(int_c);
        }
    }

    fn push_literal<T: GeneralInterface + Clone>(literal: T) -> ByteCode {
        ByteCode::Operate(
            0,
            Box::new(move |context| {
                let literal = context.allocate(Arc::new(literal.clone()));
                context.push_result(literal);
            }),
        )
    }

    fn assert_top<T: GeneralInterface + Eq>(expect: T) -> ByteCode {
        ByteCode::Operate(
            1,
            Box::new(move |context| {
                assert_eq!(
                    context
                        .inspect(context.get_argument(0))
                        .as_ref()
                        .downcast_ref(),
                    Some(&expect)
                );
            }),
        )
    }

    #[test]
    fn add_two_closure() {
        let mut interp = Interpreter::new();
        let closure_symbol = || String::from("(closure)");
        interp.load_module(Module {
            id: main_module(),
            symbol_table: [(start_symbol(), 0), (closure_symbol(), 14)]
                .into_iter()
                .collect(),
            program: vec![
                push_literal(ClosureMeta {
                    dispatch: Dispatch {
                        module_id: main_module(),
                        symbol: closure_symbol(),
                    },
                    n_capture: 1,
                }),
                push_literal(I32(2)),
                // [add two]
                // ByteCode::Operate(2, Box::new(Closure::operate_new)),
                // 1 [add two]
                push_literal(I32(1)),
                // [dispatch] [capture pack] 1 [add two]
                ByteCode::Operate(2, Box::new(Closure::operate_apply)),
                // [add two](1) [add two]
                ByteCode::Call(2),
                ByteCode::AssertFloating(1),
                assert_top(I32(3)),
                // [add two]
                ByteCode::Copy(2),
                push_literal(I32(40)),
                ByteCode::Operate(2, Box::new(Closure::operate_apply)),
                ByteCode::Call(2),
                assert_top(I32(42)),
                ByteCode::Return(0),
                // (closure)
                ByteCode::Unpack,
                ByteCode::AssertFloating(2),
                ByteCode::Operate(2, Box::new(I32::operate_add_two)),
                ByteCode::Return(1),
            ],
        });
        interp.push_call(start_dispatch(), 0);
        while interp.has_step() {
            interp.step();
        }
    }

    #[test]
    fn always_ready() {
        let mut interp = Interpreter::new();
        let poll_symbol = String::from("(poll)");
        interp.load_module(Module {
            id: main_module(),
            symbol_table: [(start_symbol(), 0), (poll_symbol.clone(), 8)]
                .into_iter()
                .collect(),
            program: vec![
                push_literal(ClosureMeta {
                    dispatch: Dispatch {
                        module_id: main_module(),
                        symbol: poll_symbol.clone(),
                    },
                    n_capture: 0,
                }),
                // ByteCode::Operate(1, Box::new(Closure::operate_new)),
                ByteCode::Operate(1, Box::new(Closure::operate_apply)),
                ByteCode::Call(1),
                ByteCode::PackFloating(1),
                // ByteCode::Operate(3, Box::new(Closure::operate_poll)),
                ByteCode::Copy(3),
                ByteCode::Return(2),
                // (poll)
                ByteCode::Unpack,
                ByteCode::AssertFloating(0),
                push_literal(List(Vec::new())),
                ByteCode::Operate(1, Box::new(Ready::operate_new)),
                ByteCode::Return(1),
            ],
        });
        interp.push_call(start_dispatch(), 0);
        while interp.has_step() {
            interp.step();
        }
        let result_list = interp.reset();
        assert_eq!(result_list.len(), 2);
        //     assert_eq!(
        //         interp
        //             .collector
        //             .inspect(result_list[0])
        //             .as_ref()
        //             .downcast_ref(),
        //         Some(&True)
        //     );
    }

    // #[derive(Debug)]
    // struct Notify(Mutex<bool>);
    // impl LeafObject for Notify {}
    // impl Notify {
    //     fn operate_poll(context: &mut dyn OperateContext) {
    //         let notify = context.get_argument(0);
    //         let notify: &Notify = context.inspect(notify).as_ref().downcast_ref().unwrap();
    //         let signal = *notify.0.lock().unwrap();
    //         let result: Handle = if signal {
    //             let unit = context.allocate(Arc::new(List(Vec::new())));
    //             Arc::new(Ready(unit))
    //         } else {
    //             Arc::new(Pending)
    //         };
    //         let result = context.allocate(result);
    //         context.push_result(result);
    //     }
    // }

    // #[test]
    // fn ready_on_notify() {
    //     let mut interp = Interpreter::new();
    //     let poll_symbol = String::from("(poll)");
    //     interp.load_module(Module {
    //         id: main_module(),
    //         symbol_table: [(start_symbol(), 0), (poll_symbol.clone(), 7)]
    //             .into_iter()
    //             .collect(),
    //         program: vec![
    //             // async closure will be pushed externally
    //             ByteCode::AssertFloating(1),
    //             ByteCode::Operate(1, Box::new(Closure::operate_apply)),
    //             ByteCode::Call(1),
    //             ByteCode::PackFloating(1),
    //             ByteCode::Operate(3, Box::new(Closure::operate_poll)),
    //             ByteCode::Copy(3),
    //             ByteCode::Return(2),
    //             // (poll)
    //             ByteCode::Unpack,
    //             ByteCode::AssertFloating(1),
    //             ByteCode::Operate(1, Box::new(Notify::operate_poll)),
    //             // we don't actually need to capture Notify again on Ready, but anyway
    //             ByteCode::Copy(2),
    //             ByteCode::Return(2),
    //         ],
    //     });
    //     let notify_handle = Arc::new(Notify(Mutex::new(false)));
    //     let notify = interp.collector.allocate(notify_handle.clone());
    //     let notify_closure = Closure {
    //         dispatch: Dispatch {
    //             module_id: main_module(),
    //             symbol: poll_symbol.clone(),
    //         },
    //         capture_list: vec![notify],
    //     };
    //     let notify_closure = interp.collector.allocate(Arc::new(notify_closure));

    //     let run_closure = |interp: &mut Interpreter| {
    //         interp.push_variable(notify_closure);
    //         interp.push_call(start_dispatch(), 0);
    //         while interp.has_step() {
    //             interp.step();
    //         }
    //         interp.reset()
    //     };
    //     for _ in 0..3 {
    //         let result_list = run_closure(&mut interp);
    //         assert_eq!(
    //             interp
    //                 .collector
    //                 .inspect(result_list[0])
    //                 .as_ref()
    //                 .downcast_ref(),
    //             Some(&False)
    //         );
    //     }
    //     *notify_handle.0.lock().unwrap() = true;
    //     let result_list = run_closure(&mut interp);
    //     assert_eq!(
    //         interp
    //             .collector
    //             .inspect(result_list[0])
    //             .as_ref()
    //             .downcast_ref(),
    //         Some(&True)
    //     );
    // }
}
*/
