use crate::collector::Collector;
use crate::objects::{Dispatch, False, List, True};
use crate::{Address, GeneralInterface, Handle};
use std::collections::HashMap;
use std::sync::Arc;

pub enum ByteCode {
    Copy(u8),
    Operate(u8, Box<dyn Fn(&mut dyn OperateContext)>),
    Jump(i8), // jump if stack top is true, by instruction offset
    Call(u8), // push calling frame according to Dispatch on stack top
    Return(u8),
    AssertFloating(u8), // assert number of floating variables
    PackFloating(u8),   // pack remaining variables into one single variable
    Unpack,             // unpack List on stack top
}

pub type ModuleId = String;

pub trait OperateContext {
    fn inspect(&self, address: Address) -> &dyn GeneralInterface;
    fn inspect_mut(&mut self, address: Address) -> &mut dyn GeneralInterface;
    fn allocate(&mut self, handle: Handle) -> Address;
    fn get_argument(&self, index: u8) -> Address;
    fn push_result(&mut self, address: Address);
}

pub struct Module {
    pub id: ModuleId,
    pub program: Vec<ByteCode>,
    pub symbol_table: HashMap<String, usize>,
}

pub struct Interpreter {
    collector: Collector,
    module_table: HashMap<ModuleId, Module>,
    variable_stack: Vec<Address>,
    call_stack: Vec<Frame>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

struct Frame {
    pointer: (ModuleId, usize),
    stack_size: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            collector: Collector::new(),
            module_table: HashMap::new(),
            variable_stack: Vec::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn load_module(&mut self, module: Module) {
        self.module_table.insert(module.id.clone(), module);
    }

    pub fn push_call(&mut self, dispatch: Dispatch) {
        let offset = *self
            .module_table
            .get(&dispatch.module_id)
            .unwrap()
            .symbol_table
            .get(&dispatch.symbol)
            .unwrap();
        self.call_stack.push(Frame {
            pointer: (dispatch.module_id, offset),
            stack_size: self.variable_stack.len(),
        });
    }

    pub fn has_step(&self) -> bool {
        !self.call_stack.is_empty()
    }

    pub fn garbage_collect(&mut self) {
        self.collector.mark_copy(&self.variable_stack);
    }
}

struct Context<'i> {
    collector: &'i mut Collector,
    variable_stack: &'i mut Vec<Address>,
    argument_offset: usize,
}

impl<'i> OperateContext for Context<'i> {
    fn inspect(&self, address: Address) -> &dyn GeneralInterface {
        self.collector.inspect(address)
    }
    fn inspect_mut(&mut self, address: Address) -> &mut dyn GeneralInterface {
        self.collector.inspect_mut(address)
    }
    fn allocate(&mut self, handle: Handle) -> Address {
        self.collector.allocate(handle)
    }
    fn get_argument(&self, index: u8) -> Address {
        self.variable_stack[self.argument_offset + index as usize]
    }
    fn push_result(&mut self, address: Address) {
        self.variable_stack.push(address);
    }
}

impl Interpreter {
    pub fn step(&mut self) {
        let pointer = &mut self.call_stack.last_mut().unwrap().pointer;
        let instruction = &self.module_table.get(&pointer.0).unwrap().program[pointer.1];
        pointer.1 += 1;
        match instruction {
            ByteCode::Copy(offset) => {
                self.variable_stack
                    .push(self.variable_stack[self.variable_stack.len() - *offset as usize]);
            }
            ByteCode::Operate(n_argument, op) => {
                let argument_offset = self.variable_stack.len() - *n_argument as usize;
                op(&mut Context {
                    collector: &mut self.collector,
                    variable_stack: &mut self.variable_stack,
                    argument_offset,
                });
            }
            ByteCode::Jump(offset) => {
                let top = *self.variable_stack.last().unwrap();
                let top = self.collector.inspect(top);
                if top.as_ref().is::<True>() {
                    let pointer = &mut self.call_stack.last_mut().unwrap().pointer;
                    if *offset > 0 {
                        pointer.1 += *offset as usize;
                    } else {
                        pointer.1 -= (-*offset) as usize;
                    }
                } else if !top.as_ref().is::<False>() {
                    panic!("jump on non-boolean variable {:?}", top);
                }
            }
            ByteCode::Call(n_argument) => {
                let dispatch = *self.variable_stack.last().unwrap();
                let dispatch: &Dispatch = self
                    .collector
                    .inspect(dispatch)
                    .as_ref()
                    .downcast_ref()
                    .unwrap();
                let dispatch = dispatch.clone();
                self.variable_stack.remove(self.variable_stack.len() - 1); // is it useful to save it?
                let stack_size = self.variable_stack.len() - *n_argument as usize;
                self.call_stack.last_mut().unwrap().stack_size = stack_size;
                self.push_call(dispatch);
                self.call_stack.last_mut().unwrap().stack_size = stack_size;
            }
            ByteCode::Return(n_returned) => {
                let n_returned = *n_returned;
                self.call_stack.pop();
                let stack_size = self
                    .call_stack
                    .last()
                    .map(|frame| frame.stack_size)
                    .unwrap_or(0);
                self.variable_stack
                    .drain(stack_size..self.variable_stack.len() - n_returned as usize);
            }
            ByteCode::AssertFloating(n_floating) => {
                assert_eq!(
                    self.variable_stack.len() - self.call_stack.last().unwrap().stack_size,
                    *n_floating as usize
                );
            }
            ByteCode::PackFloating(n_destructed) => {
                let n_destructed = *n_destructed;
                let stack_size = self.call_stack.last().unwrap().stack_size;
                assert!(self.variable_stack.len() - stack_size >= n_destructed as usize);
                let pack_offset = self.variable_stack.len() - (stack_size + n_destructed as usize);
                let list = List((&self.variable_stack[pack_offset..]).to_vec());
                let list = self.collector.allocate(Arc::new(list));
                self.variable_stack.drain(pack_offset..);
                self.variable_stack.push(list);
            }
            ByteCode::Unpack => {
                let pack = *self.variable_stack.last().unwrap();
                let pack: &List = self
                    .collector
                    .inspect(pack)
                    .as_ref()
                    .downcast_ref()
                    .unwrap();
                self.variable_stack.pop();
                self.variable_stack.extend(&pack.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::LeafObject;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref MAIN_MODULE: ModuleId = String::from("main");
        static ref START_SYMBOL: String = String::from("start");
        static ref START_DISPATCH: Dispatch = Dispatch {
            module_id: MAIN_MODULE.clone(),
            symbol: START_SYMBOL.clone(),
        };
    }

    #[test]
    fn simple_step() {
        let mut interp = Interpreter::new();
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            program: vec![ByteCode::Return(0)],
            symbol_table: [(START_SYMBOL.clone(), 0)].into_iter().collect(),
        });
        interp.push_call(START_DISPATCH.clone());
        assert!(interp.has_step());
        interp.step();
        assert!(!interp.has_step());
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct I32(i32);
    impl LeafObject for I32 {}

    fn operate_add_two_i32(context: &mut dyn OperateContext) {
        let int_a = context.get_argument(0);
        let int_a: I32 = *context.inspect(int_a).as_ref().downcast_ref().unwrap();
        let int_b = context.get_argument(1);
        let int_b: I32 = *context.inspect(int_b).as_ref().downcast_ref().unwrap();
        let int_c = context.allocate(Arc::new(I32(int_a.0 + int_b.0)));
        context.push_result(int_c);
    }

    #[test]
    fn add_two_i32() {
        let mut interp = Interpreter::new();
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            symbol_table: [(START_SYMBOL.clone(), 0)].into_iter().collect(),
            program: vec![
                push_literal(I32(20)),
                push_literal(I32(22)),
                ByteCode::Operate(2, Box::new(operate_add_two_i32)),
                assert_top(I32(42)),
                ByteCode::Return(0),
            ],
        });
        interp.push_call(START_DISPATCH.clone());
        while interp.has_step() {
            interp.step();
        }
    }

    fn operate_add_i32_in_place(context: &mut dyn OperateContext) {
        let int_a = context.get_argument(0);
        let int_a: I32 = *context.inspect(int_a).as_ref().downcast_ref().unwrap();
        let int_b = context.get_argument(1);
        let int_b: I32 = *context.inspect(int_b).as_ref().downcast_ref().unwrap();
        let int_c = I32(int_a.0 + int_b.0);
        let int_b = context.get_argument(1);
        *context.inspect_mut(int_b).as_mut().downcast_mut().unwrap() = int_c;
    }

    #[test]
    fn add_i32_in_place() {
        let mut interp = Interpreter::new();
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            symbol_table: [(START_SYMBOL.clone(), 0)].into_iter().collect(),
            program: vec![
                push_literal(I32(20)),
                push_literal(I32(22)),
                ByteCode::Operate(2, Box::new(operate_add_i32_in_place)),
                assert_top(I32(42)),
                ByteCode::Copy(2),
                assert_top(I32(20)),
                ByteCode::Return(0),
            ],
        });
        interp.push_call(START_DISPATCH.clone());
        while interp.has_step() {
            interp.step();
        }
    }

    fn operate_eq_two_i32(context: &mut dyn OperateContext) {
        let int_a = context.get_argument(0);
        let int_a: I32 = *context.inspect(int_a).as_ref().downcast_ref().unwrap();
        let int_b = context.get_argument(1);
        let int_b: I32 = *context.inspect(int_b).as_ref().downcast_ref().unwrap();
        let result: Handle = if int_a == int_b {
            Arc::new(True)
        } else {
            Arc::new(False)
        };
        let result = context.allocate(result);
        context.push_result(result);
    }

    #[test]
    fn fib_10() {
        // in a very wasteful way...
        let mut interp = Interpreter::new();
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            symbol_table: [(START_SYMBOL.clone(), 0)].into_iter().collect(),
            program: vec![
                push_literal(I32(10)), // n
                push_literal(I32(-1)), // _
                push_literal(I32(0)),  // b
                push_literal(I32(1)),  // a
                push_literal(I32(1)),  // 1
                push_literal(I32(1)),  // i
                push_literal(I32(-1)), // _
                // 'loop: T i' 1 a' a ? n => _ i 1 a b _ n
                // i _ i 1 a b _ n
                ByteCode::Copy(2),
                // n i _ i 1 a b
                ByteCode::Copy(8),
                // ? n i _ i 1 a b
                ByteCode::Operate(2, Box::new(operate_eq_two_i32)),
                // goto 'end
                ByteCode::Jump(8),
                // a ? n i _ i 1 a b
                ByteCode::Copy(7),
                // b a ? n i _ i 1
                ByteCode::Copy(9),
                // a' a ? n i _ i 1
                ByteCode::Operate(2, Box::new(operate_add_i32_in_place)),
                // 1 a' a ? n i
                ByteCode::Copy(8),
                // i 1 a' a ? n
                ByteCode::Copy(6),
                // i' 1 a' a ? n
                ByteCode::Operate(2, Box::new(operate_add_i32_in_place)),
                // T i' 1 a' b ? n
                push_literal(True),
                // goto 'loop
                ByteCode::Jump(-12),
                // 'end: ? n i _ i 1 a
                ByteCode::Copy(7),
                assert_top(I32(55)),
                ByteCode::Return(0),
            ],
        });
        interp.push_call(START_DISPATCH.clone());
        while interp.has_step() {
            interp.step();
        }
    }

    #[test]
    fn fib_10_recursive() {
        // in a very naive way...
        let mut interp = Interpreter::new();
        let fib_symbol = String::from("fib");
        let fib_dispatch = Dispatch {
            module_id: MAIN_MODULE.clone(),
            symbol: fib_symbol.clone(),
        };
        interp.load_module(Module {
            id: MAIN_MODULE.clone(),
            symbol_table: [(START_SYMBOL.to_string(), 0), (fib_symbol.clone(), 6)]
                .into_iter()
                .collect(),
            program: vec![
                push_literal(I32(10)),
                push_literal(fib_dispatch.clone()),
                ByteCode::Call(1),
                ByteCode::AssertFloating(1),
                assert_top(I32(55)),
                ByteCode::Return(0),
                // fib
                // n
                ByteCode::AssertFloating(1),
                // 1 n
                push_literal(I32(1)),
                // ? 1 n
                ByteCode::Operate(2, Box::new(operate_eq_two_i32)),
                // goto '1
                ByteCode::Jump(19),
                // n ? 1
                ByteCode::Copy(3),
                // 2 n
                push_literal(I32(2)),
                // ? 2 n
                ByteCode::Operate(2, Box::new(operate_eq_two_i32)),
                // goto '2
                ByteCode::Jump(15),
                // -1 ? 2 n
                push_literal(I32(-1)),
                // n -1
                ByteCode::Copy(4),
                // n' n
                ByteCode::Operate(2, Box::new(operate_add_two_i32)),
                push_literal(fib_dispatch.clone()),
                ByteCode::Call(1),
                ByteCode::AssertFloating(1),
                // -2 fib(n') n
                push_literal(I32(-2)),
                // n -2 fib(n')
                ByteCode::Copy(3),
                // n'' n -2 fib(n')
                ByteCode::Operate(2, Box::new(operate_add_two_i32)),
                push_literal(fib_dispatch.clone()),
                // fib(n'') n -2 fib(n')
                ByteCode::Call(1),
                ByteCode::AssertFloating(1),
                // fib(n') fib(n'')
                ByteCode::Copy(4),
                ByteCode::Operate(2, Box::new(operate_add_two_i32)),
                ByteCode::Return(1),
                // '1 '2
                push_literal(I32(1)),
                ByteCode::Return(1),
            ],
        });
        interp.push_call(START_DISPATCH.clone());
        while interp.has_step() {
            interp.step();
        }
    }
}
