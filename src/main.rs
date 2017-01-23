use std::collections::HashMap;
use std::fmt;
use std::fmt::{Write};

use std::cmp; //for max

use std::io; //for IO


type Addr = i32;
type Name = String;

type CoreVariable = Name;

#[derive(Clone, PartialEq, Eq, Debug)]
struct CoreLet {
    is_rec: bool,
    bindings: Vec<(Name, Box<CoreExpr>)>,
    expr: Box<CoreExpr>
}



#[derive(Clone, PartialEq, Eq)]
enum CoreExpr {
    //change this?
    Variable(Name),
    Num(i32),
    Application(Box<CoreExpr>, Box<CoreExpr>),
    Pack{tag: u32, arity: u32},
    Let(CoreLet),


}

impl fmt::Debug for CoreExpr {

    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &CoreExpr::Variable(ref name) => write!(fmt, "{}", name),
            &CoreExpr::Num(ref num) => write!(fmt, "n_{}", num),
            &CoreExpr::Application(ref e1, ref e2) =>
            write!(fmt, "({:#?} $ {:#?})", *e1, *e2),
            &CoreExpr::Let(CoreLet{ref is_rec, ref bindings, ref expr}) => {
                if *is_rec {
                    try!(write!(fmt, "letrec"));
                } else {
                    try!(write!(fmt, "let"));
                }
                try!(write!(fmt, " {{\n"));
                for &(ref name, ref expr) in bindings {
                    try!(write!(fmt, "{} = {:#?}\n", name, expr));
                }
                try!(write!(fmt, "in\n"));
                try!(write!(fmt, "{:#?}", expr));
                write!(fmt, "}}")
            }
            &CoreExpr::Pack{ref tag, ref arity} => {
                write!(fmt, "Pack(tag: {} arity: {})", tag, arity)
            }
        }
    }
}


#[derive(Clone, PartialEq, Eq)]
struct SupercombDefn {
    name: String,
    args: Vec<String>,
    body: CoreExpr
}


impl fmt::Debug for SupercombDefn {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(fmt, "{} ", &self.name));
        for arg in self.args.iter() {
            try!(write!(fmt, "{} ", &arg));
        }
        try!(write!(fmt, "{{ {:#?} }}", self.body));
        Result::Ok(())

    }

}


//a core program is a list of supercombinator
//definitions
type CoreProgram = Vec<SupercombDefn>;

//primitive operations on the machine
#[derive(Clone, PartialEq, Eq)]
enum MachinePrimOp {
    Add,
    Sub,
    Mul,
    Div,
    Negate,
    G,
    GEQ,
    L,
    LEQ,
    EQ,
    NEQ,
    Construct {
        tag: DataTag,
        arity: u32
    },
    If
}

impl fmt::Debug for MachinePrimOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
           &MachinePrimOp::Negate => write!(fmt, "Negate"),
           &MachinePrimOp::Add => write!(fmt, "+"),
           &MachinePrimOp::Sub => write!(fmt, "-"),
           &MachinePrimOp::Mul => write!(fmt, "*"),
           &MachinePrimOp::Div => write!(fmt, "/"),
           &MachinePrimOp::G => write!(fmt, ">"),
           &MachinePrimOp::L => write!(fmt, "<"),
           &MachinePrimOp::GEQ => write!(fmt, ">="),
           &MachinePrimOp::LEQ => write!(fmt, "<="),
           &MachinePrimOp::EQ => write!(fmt, "=="),
           &MachinePrimOp::NEQ => write!(fmt, "!="),
           &MachinePrimOp::If => write!(fmt, "if"),
           &MachinePrimOp::Construct{tag, arity} => {
                write!(fmt, "Construct-tag:{} | arity: {}", tag, arity)
            }
        }
    }
}

type DataTag = u32;

//heap nodes
#[derive(Clone, PartialEq, Eq)]
enum HeapNode {
    Application {
        fn_addr: Addr,
        arg_addr: Addr
    },
    Supercombinator(SupercombDefn),
    Num(i32),
    Indirection(Addr),
    Primitive(MachinePrimOp),
    Data{tag: DataTag, component_addrs: Vec<Addr>}
}

impl fmt::Debug for HeapNode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &HeapNode::Application{ref fn_addr, ref arg_addr} => {
                write!(fmt, "H-({} $ {})", fn_addr, arg_addr)
            }
            &HeapNode::Supercombinator(ref sc_defn) => {
                write!(fmt, "H-supcomb-{:#?}", sc_defn)
            },
            &HeapNode::Num(ref num)  => {
                write!(fmt, "H-{}", num)
            }
            &HeapNode::Indirection(ref addr)  => {
                write!(fmt, "H-indirection-{}", addr)
            }
            &HeapNode::Primitive(ref primop)  => {
                write!(fmt, "H-prim-{:#?}", primop)
            },
            &HeapNode::Data{ref tag, ref component_addrs} => {
                write!(fmt, "H-data: tag: {} addrs: {:#?}", tag, component_addrs)
            }
        }
    }
}

impl HeapNode {
    fn is_data_node(&self) -> bool {
        match self {
            &HeapNode::Num(_) => true,
            &HeapNode::Data{..} => true,
            _ => false
        }
    }

}

fn unwrap_heap_node_to_ap(node: HeapNode) -> Result<(Addr, Addr), MachineError> {
    match node {
        HeapNode::Application{fn_addr, arg_addr} => 
            Result::Ok((fn_addr, arg_addr)),
        other @ _ => Result::Err(format!(
                "expected application node, found: {:#?}", other))
    }
}

//unsued for mark 1
// a dump is a vector of stacks
type Dump = Vec<Stack>;

//stack of addresses of nodes. "Spine"
#[derive(Clone,PartialEq,Eq,Debug)]
struct Stack {
    stack: Vec<Addr>
}

impl Stack {
    fn new() -> Stack {
        Stack {
            stack: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.stack.len()
    }

    fn push(&mut self, addr: Addr) {
        self.stack.push(addr)
    }

    fn pop(&mut self) -> Addr {
        self.stack.pop().expect("top of stack is empty")
    }

    fn peek(&self) -> Addr {
        self.stack.last().expect("top of stack is empty to peek").clone()
    }

    fn iter(&self) -> std::slice::Iter<Addr> {
        self.stack.iter()
    }

}

//maps names to addresses in the heap
type Bindings = HashMap<Name, Addr>;

//maps addresses to machine Nodes
#[derive(Clone)]
struct Heap {
    heap: HashMap<Addr, HeapNode>,
    next_addr: Addr
}

impl fmt::Debug for Heap {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut keyvals : Vec<(&Addr, &HeapNode)> = self.heap.iter().collect();
        keyvals.sort_by(|a, b| a.0.cmp(b.0));

        for &(key, val) in keyvals.iter().rev() {
            try!(write!(fmt, "\t{} => {:#?}\n", key, val));
        }

        return Result::Ok(())

    }
}

impl Heap {
    fn new()  -> Heap {
        Heap {
            heap: HashMap::new(),
            next_addr: 0
        }
    }

    //allocate the HeapNode on the heap
    fn alloc(&mut self, node: HeapNode) -> Addr {
        let addr = self.next_addr;
        self.next_addr += 1;

        self.heap.insert(addr, node);
        addr
    }

    fn get(&self, addr: &Addr) -> HeapNode {
        self.heap
        .get(&addr)
        .cloned()
        .expect(&format!("expected heap node at addess: {}", addr))
    }

    fn rewrite(&mut self, addr: &Addr, node: HeapNode) {
        assert!(self.heap.contains_key(addr),
                "asked to rewrite (address: {}) with (node: {:#?}) which does not exist on heap",
                addr, node);
        self.heap.insert(*addr, node);
    }

}

//state of the machine
#[derive(Clone)]
struct MachineOptions {
    update_heap_on_sc_eval: bool,
}

#[derive(Clone)]
struct Machine {
    stack : Stack,
    heap : Heap,
    globals: Bindings,
    dump: Dump,
    options: MachineOptions,
}

type MachineError = String;



fn format_heap_node(m: &Machine, env: &Bindings, node: &HeapNode) -> String {
    match node {
        &HeapNode::Indirection(addr) => format!("indirection: {}", addr),
        &HeapNode::Num(num) => format!("{}", num),
        &HeapNode::Primitive(ref primop) => format!("prim-{:#?}", primop),
        &HeapNode::Application{ref fn_addr, ref arg_addr} =>
        format!("({} $ {})",
                format_heap_node(m, env, &m.heap.get(fn_addr)),
                format_heap_node(m, env, &m.heap.get(arg_addr))),
        &HeapNode::Supercombinator(ref sc_defn) =>  {
            let mut sc_str = String::new();
            write!(&mut sc_str, "{}", sc_defn.name).unwrap();
            sc_str
        }
        &HeapNode::Data{ref tag, ref component_addrs} => {
            let mut data_str = String::new();
            data_str  += &format!("data-{}", tag);
            for c in component_addrs.iter() {
                data_str += 
                       &format!("{}", format_heap_node(m, env, &m.heap.get(c)))
            }
            data_str
        }

    }
}

fn print_machine(m: &Machine, env: &Bindings) {
    fn print_stack(m: &Machine, env: &Bindings, s: &Stack) {
        print!( "stack:\n");
        print!( "## top ##\n");
        for addr in s.iter().rev() {
            print!("heap[{}] :  {}\n",
                   *addr,
                   format_heap_node(m,
                                    env,
                                    &m.heap.get(addr)));
        }
        print!( "## bottom ##\n");
    };

    print_stack(m, env, &m.stack);

    print!("*** heap: ***\n");
    print!("{:#?}", m.heap);

    print!("*** dump: ***\n");
    for stack in  m.dump.iter().rev() {
        print_stack(m, env, stack);
    }
}

fn bool_to_heap_node(b: bool) -> HeapNode {
    if b {
       HeapNode::Primitive(MachinePrimOp::Construct{tag: 1, arity: 0})
    }
    else {
        HeapNode::Primitive(MachinePrimOp::Construct{tag: 0, arity: 0})
    }

}


fn get_prelude() -> CoreProgram {
    string_to_program("I x = x;\
                      K x y = x;\
                      K1 x y = y;\
                      S f g x = f x (g x);\
                      compose f g x = f (g x);\
                      twice f = compose f f;\
                      False = Pack{0, 0};\
                      True = Pack{1, 0}\
                      ".to_string()).unwrap()
}

fn get_primitives() -> Vec<(Name, MachinePrimOp)> {
    [("+".to_string(), MachinePrimOp::Add),
    ("-".to_string(), MachinePrimOp::Sub),
    ("*".to_string(), MachinePrimOp::Mul),
    ("/".to_string(), MachinePrimOp::Div),
    (">".to_string(), MachinePrimOp::G),
    ("<".to_string(), MachinePrimOp::L),
    (">=".to_string(), MachinePrimOp::GEQ),
    ("<=".to_string(), MachinePrimOp::LEQ),
    ("!=".to_string(), MachinePrimOp::NEQ),
    ("==".to_string(), MachinePrimOp::EQ),
    ("negate".to_string(), MachinePrimOp::Negate),
    ("if".to_string(), MachinePrimOp::If),
    ].iter().cloned().collect()
}

fn heap_build_initial(sc_defs: CoreProgram, prims: Vec<(Name, MachinePrimOp)>) -> (Heap, Bindings) {
    let mut heap = Heap::new();
    let mut globals = HashMap::new();

    for sc_def in sc_defs.iter() {
        //create a heap node for the supercombinator definition
        //and insert it
        let node = HeapNode::Supercombinator(sc_def.clone());
        let addr = heap.alloc(node);

        //insert it into the globals, binding the name to the
        //heap address
        globals.insert(sc_def.name.clone(), addr);
    }

    for (name, prim_op) in prims.into_iter() {
        let addr = heap.alloc(HeapNode::Primitive(prim_op));
        globals.insert(name, addr);
    }

    (heap, globals)
}


// *** INTERPRETER ***
//interreter

impl Machine {
    fn new(program: CoreProgram) -> Machine {
        //all supercombinators = program + prelude
        let mut sc_defs = program.clone();
        sc_defs.extend(get_prelude().iter().cloned());

        let (initial_heap, globals) = heap_build_initial(sc_defs,
                                                         get_primitives());

        //get main out of the heap
        let main_addr : Addr = match globals.get("main") {
            Some(main) => main,
            None => panic!("no main found")
        }.clone();

        Machine {
            dump: Vec::new(),
            //stack has addr main on top
            stack:  {
                let mut s = Stack::new();
                s.push(main_addr);
                s
            },
            globals: globals,
            heap: initial_heap,
            options: MachineOptions {
                update_heap_on_sc_eval: true
            }
        }
    }

    //returns bindings of this run
    fn step(&mut self) -> Result<Bindings, MachineError>{
        //top of stack
        let tos_addr : Addr = self.stack.peek();
        let heap_val = self.heap.get(&tos_addr);


        //there is something on the dump that wants to use this
        //data node, so pop it back.
        if heap_val.is_data_node() && self.dump.len() > 0 {
            self.stack = self.dump
            .pop()
            .expect("dump should have at least 1 element");
            Result::Ok(self.globals.clone())
        } else {
            self.run_step(&heap_val)
        }
    }

    //make an environment for the execution of the supercombinator
    fn make_supercombinator_env(sc_defn: &SupercombDefn,
                                heap: &Heap,
                                stack_args:&Vec<Addr>,
                                globals: &Bindings) -> 
        Result<Bindings, MachineError> {

        assert!(stack_args.len() == sc_defn.args.len());

        let mut env = globals.clone();

        /*
         * let f a b c = <body>
         *
         * if a function call of the form f x y z was made,
         * the stack will look like
         * ---top---
         * f
         * f x
         * f x y
         * f x y z
         * --------
         *
         * the "f" will be popped beforehand (that is part of the contract
         * of calling make_supercombinator_env)
         *
         *
         * So, we go down the stack, removing function applications, and
         * binding the RHS to the function parameter names.
         *
         */
         for (arg_name, application_addr) in
         sc_defn.args.iter().zip(stack_args.iter()) {

            let application = heap.get(application_addr);
            let (_, param_addr) = try!(unwrap_heap_node_to_ap(application));
            env.insert(arg_name.clone(), param_addr);

        }
        Result::Ok(env)
    }
    

    fn run_primitive_negate(&mut self) -> Result<(), MachineError> {
        //we need a copy of the stack to push into the dump
        let stack_copy = self.stack.clone();

        //pop the primitive off
        self.stack.pop();

        //we rewrite this addres in case of
        //a raw number
        let neg_ap_addr = self.stack.peek();

        //Apply <negprim> <argument>
        //look at what argument is and dispatch work
        let to_negate_val = 
            match try!(setup_heap_node_access(self,
                                              stack_copy, 
                                              neg_ap_addr,
                                              heap_try_num_access)) {
            HeapAccessValue::Found(val) => val,
            HeapAccessValue::SetupExecution => return Result::Ok(())
        };

        self.heap.rewrite(&neg_ap_addr, HeapNode::Num(-to_negate_val));
        Result::Ok(())
    }


    //extractor should return an error if a node cannot have data
    //extracted from. It should return None
    fn run_primitive_num_binop<F>(&mut self, handler: F) -> Result<(), MachineError> 
    where F: Fn(i32, i32) -> HeapNode {

        let stack_copy = self.stack.clone();

        //stack will be like

        //top--v
        //+
        //(+ a)
        //(+ a) b
        //bottom-^

        //fully eval a, b
        //then do stuff

        //pop off operator
        self.stack.pop();


        let left_value = {
            //pop off left value
            let left_ap_addr = self.stack.pop();
            match try!(setup_heap_node_access(self,
                                                   stack_copy.clone(),
                                                    left_ap_addr,
                                                    heap_try_num_access)) {
                HeapAccessValue::Found(val) => val,
                HeapAccessValue::SetupExecution => return Result::Ok(())
            }
        };

        //do the same process for right argument
        //peek (+ a) b
        //we peek, since in the case where (+ a) b can be reduced,
        //we simply rewrite the node (+ a b) with the final value
        //(instead of creating a fresh node)
        let binop_ap_addr = self.stack.peek();
        let right_value = 
            match try!(setup_heap_node_access(self, 
                                              stack_copy,
                                              binop_ap_addr,
                                              heap_try_num_access)) {
                HeapAccessValue::Found(val) => val,
                HeapAccessValue::SetupExecution => return Result::Ok(())
            };

        self.heap.rewrite(&binop_ap_addr, handler(left_value,
                                                  right_value));

        Result::Ok(())
    } //close fn

    //TODO: find out what happens when constructor of arity 0 is
    //called
    fn run_constructor(&mut self,
                       tag: DataTag,
                       arity: u32) -> Result<(), MachineError> {

        //pop out constructor
        //TODO: check if this is legit: before, I used to pop this.
        //Now, I'm rewriting this address. This _should_ work, but I'm
        //not 100% sure
        self.stack.pop();

        if self.stack.len() < arity as usize {
            return Result::Err(format!("expected to have \
                                       {} arguments to {} \
                                       constructor, found {}",
                                       arity, 
                                       tag,
                                       self.stack.len()));
        }
    
        let mut arg_addrs : Vec<Addr> = Vec::new();
        
        //This will be rewritten with the data
        //since the fn call would have been something like:
        //##top##
        //(Prim (Constructor tag arity))
        //(Prim (Constructor tag arity) $ a)
        //(Prim (Constructor tag arity) a $ b)
        //( Prim (Constructor tag arity) a b $ c) <- to rewrite
        //##bottom##

        for _ in 0..arity {
            let (_, arg_addr) = try!(unwrap_heap_node_to_ap(self.heap.get(&self.stack.pop())));
            arg_addrs.push(arg_addr);
        };

        let new_alloc_addr = self.heap.alloc(HeapNode::Data{
                              component_addrs: arg_addrs,
                               tag: tag
                          });
        self.stack.push(new_alloc_addr);
        Result::Ok(())
    }
    fn dump_stack(&mut self, stack: Stack) {
        self.dump.push(stack);
        self.stack = Stack::new();
    }
    
    fn run_primitive_if(&mut self) -> Result<(), MachineError> {
        let stack_copy = self.stack.clone();

        //remove if condition
        self.stack.pop();

        //## top of stack
        //if 
        //if $ <cond> <- if_ap_addr
        //if <cond> $ <then>
        //if <cond> <then> $ <else>
        //## bottom of stack
        let if_ap_addr = self.stack.peek();
        println!("if_ap_addr: {}", if_ap_addr);

        let then_ap_addr = try!(self.stack.clone()
                                .iter()
                                .nth(1)
                                .ok_or("expected then application, was not found on stack".to_string())).clone();
        println!("then_ap_addr: {}", then_ap_addr);

        let else_ap_addr = try!(self.stack.clone()
                            .iter()
                            .nth(2)
                            .ok_or("expected else application, was not found on stack".to_string())).clone();
        println!("else_ap_addr: {}", else_ap_addr);

        let cond : bool = {
            println!("extracting cond addr...");
            match try!(setup_heap_node_access(self,
                                              stack_copy,
                                              if_ap_addr,
                                              heap_try_bool_access)) {
                HeapAccessValue::Found(b) => b,
                HeapAccessValue::SetupExecution => {
                    println!("setting up execution...");
                    return Result::Ok(())
                }
            }
        };
        
        println!("found cond: {}", cond);
        
        if cond {
            let (_, then_addr) = try!(unwrap_heap_node_to_ap(self.heap.get(&then_ap_addr)));
            let then_node = self.heap.get(&then_addr);
            self.heap.rewrite(&if_ap_addr, then_node);
        }
        else {
            let (_, else_addr) = try!(unwrap_heap_node_to_ap(self.heap.get(&else_ap_addr)));
            let else_node = self.heap.get(&else_addr);
            self.heap.rewrite(&if_ap_addr, else_node);
        }
        Result::Ok(())
    }


    //actually run_step the computation
    fn run_step(&mut self, heap_val: &HeapNode) -> Result<Bindings, MachineError> {
        match heap_val {
            &HeapNode::Num(n) =>
                return Result::Err(format!("number applied as a function: {}", n)),
            &HeapNode::Data{..} => panic!("cannot run data node, unimplemented"),
            &HeapNode::Application{fn_addr, ..} => {
                //push function address over the function
                self.stack.push(fn_addr);
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Indirection(ref addr) => {
                //simply ignore an indirection during execution, and
                //push the indirected value on the stack
                self.stack.pop();
                self.stack.push(*addr);
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::Negate) => {
                try!(self.run_primitive_negate());
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::Add) => {
                try!(self.run_primitive_num_binop(|x, y| HeapNode::Num(x + y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::Sub) => {
                try!(self.run_primitive_num_binop(|x, y| HeapNode::Num(x - y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::Mul) => {
                try!(self.run_primitive_num_binop(|x, y| HeapNode::Num(x * y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::Div) => {
                try!(self.run_primitive_num_binop(|x, y| HeapNode::Num(x / y)));
                Result::Ok(self.globals.clone())
            }
            //construct a complex type
            &HeapNode::Primitive(MachinePrimOp::Construct {tag, arity}) => {
                try!(self.run_constructor(tag, arity));
                Result::Ok(self.globals.clone())
            }
            //boolean ops
            &HeapNode::Primitive(MachinePrimOp::G) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x > y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::GEQ) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x >= y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::L) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x < y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::LEQ) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x <= y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::EQ) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x == y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::NEQ) => {
                try!(self.run_primitive_num_binop(
                        |x, y| bool_to_heap_node(x != y)));
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Primitive(MachinePrimOp::If) => {
                try!(self.run_primitive_if());
                Result::Ok(self.globals.clone())
            }
            &HeapNode::Supercombinator(ref sc_defn) => {

                //pop the supercombinator
                let sc_addr = self.stack.pop();

                //the arguments are the stack
                //values below the supercombinator. There
                //are (n = arity of supercombinator) arguments
                let arg_addrs = {
                    let mut addrs = Vec::new();
                    for _ in 0..sc_defn.args.len() {
                        addrs.push(self.stack.pop());
                    }
                    addrs
                };

                let env = try!(Machine::make_supercombinator_env(&sc_defn,
                                                            &self.heap,
                                                            &arg_addrs,
                                                            &self.globals));

                let new_alloc_addr = try!(self.instantiate(sc_defn.body.clone(), &env));

                self.stack.push(new_alloc_addr);

                if self.options.update_heap_on_sc_eval {
                    //if the function call was (f x y), the stack will be
                    //f
                    //f x
                    //(f x) y  <- final address in arg_addrs
                    //we need to rewrite this heap value
                    let full_call_addr = {
                        //if the SC has 0 parameters (a constant), then eval the SC
                        //and replace the SC itself
                        if sc_defn.args.len() == 0 {
                            sc_addr
                        }
                        else {
                            *arg_addrs.last()
                           .expect(concat!("arguments has no final value ",
                                            "even though the supercombinator ",
                                            "has >= 1 parameter"))
                        }
                    };
                    self.heap.rewrite(&full_call_addr, HeapNode::Indirection(new_alloc_addr));
                }

                Result::Ok(env)
            }
        }
    }

    fn rebind_vars_to_env(old_addr: Addr,
                          new_addr: Addr,
                          edit_addr: Addr,
                          mut heap: &mut Heap) {

        match heap.get(&edit_addr) {
            HeapNode::Data{..} => panic!("unimplemented rebinding of Data node"),
            HeapNode::Application{fn_addr, arg_addr} => {
                let new_fn_addr = if fn_addr == old_addr {
                    new_addr
                } else {
                    fn_addr
                };


                let new_arg_addr = if arg_addr == old_addr {
                    new_addr
                } else {
                    arg_addr
                };

                //if we have not replaced, then recurse
                //into the application calls
                if fn_addr != old_addr {
                    Machine::rebind_vars_to_env(old_addr,
                                                new_addr,
                                                fn_addr,
                                                &mut heap);

                };

                if arg_addr != old_addr {
                    Machine::rebind_vars_to_env(old_addr,
                                                new_addr,
                                                arg_addr,
                                                &mut heap);
                };

                heap.rewrite(&edit_addr,
                             HeapNode::Application{
                               fn_addr: new_fn_addr,
                               arg_addr: new_arg_addr
                           });

            },
            HeapNode::Indirection(ref addr) =>
            Machine::rebind_vars_to_env(old_addr,
                                        new_addr,
                                        *addr,
                                        &mut heap),

            HeapNode::Primitive(_) => {}
            HeapNode::Supercombinator(_) => {}
            HeapNode::Num(_) => {},
        }
    }

    fn instantiate(&mut self, expr: CoreExpr, env: &Bindings) -> Result<Addr, MachineError> {
        match expr {
            CoreExpr::Let(CoreLet{expr: let_rhs, bindings, is_rec}) => {
                let mut let_env : Bindings = env.clone();

                if is_rec {
                    //TODO: change this to zip() with range

                    let mut addr = -1;
                    //first create dummy indeces for all LHS
                    for &(ref bind_name, _) in bindings.iter()  {
                        let_env.insert(bind_name.clone(), addr);
                        addr -= 1;
                    }

                    let mut old_to_new_addr: HashMap<Addr, Addr> = HashMap::new();

                    //instantiate RHS, while storing legit
                    //LHS addresses
                    //TODO: cleanup, check if into_iter is sufficient
                    for &(ref bind_name, ref bind_expr) in bindings.iter() {
                        let new_addr = try!(self.instantiate(*bind_expr.clone(), &let_env));

                        let old_addr = try!(let_env
                                            .get(bind_name)
                                            .ok_or(format!("unable to find |{}| in env", bind_name)))
                            .clone();

                        old_to_new_addr.insert(old_addr, new_addr);

                        //insert the "correct" address into the
                        //let env
                        let_env.insert(bind_name.clone(), new_addr);

                    }

                    for (old, new) in old_to_new_addr.iter() {
                        for to_edit_addr in old_to_new_addr.values() {
                            Machine::rebind_vars_to_env(*old,
                                                        *new,
                                                        *to_edit_addr,
                                                        &mut self.heap);
                        }

                    }

                    print!("letrec env:\n {:#?}", let_env);
                    self.instantiate(*let_rhs, &let_env)

                }
                else {
                    for (bind_name, bind_expr) in bindings.into_iter() {
                        let addr = try!(self.instantiate(*bind_expr, &let_env));
                        let_env.insert(bind_name.clone(), addr);
                    }
                    self.instantiate(*let_rhs, &let_env)

                }

            }
            CoreExpr::Num(x) => Result::Ok(self.heap.alloc(HeapNode::Num(x))),
            CoreExpr::Application(fn_expr, arg_expr) => {
                let fn_addr = try!(self.instantiate(*fn_expr, env));
                let arg_addr = try!(self.instantiate(*arg_expr, env));

                Result::Ok(self.heap.alloc(HeapNode::Application {
                    fn_addr: fn_addr,
                    arg_addr: arg_addr
                }))

            }
            CoreExpr::Variable(vname) => {
                match env.get(&vname) {
                    Some(addr) => Result::Ok(*addr),
                    None => Result::Err(format!("unable to find variable in heap: |{}|", vname))
                }

            }
            CoreExpr::Pack{tag, arity} => {
                let prim_for_pack = 
                    HeapNode::Primitive(MachinePrimOp::Construct{
                        tag: tag,
                        arity: arity
                    });

                Result::Ok(self.heap.alloc(prim_for_pack))

            } 
        }
    }
}


//represents what happens when you try to access a heap node for a 
//primitive run. Either you found the required heap node,
//or you ask to setup execution since there is a frozen supercombinator
//node or something else that needs to be evaluated
enum HeapAccessValue<T> {
    Found(T),
    SetupExecution
}

type HeapAccessResult<T> = Result<HeapAccessValue<T>, MachineError>;

//get a heap node of the kind that handler wants to get,
//otherwise setup the heap so that unevaluated code
//is evaluated to get something of this type
//TODO: check if we can change semantics so it does not need to take the
//application node as the parameter that's a little awkward
fn setup_heap_node_access<F, T>(m: &mut Machine,
                          stack_to_dump: Stack,
                          ap_addr: Addr,
                          access_handler: F ) -> HeapAccessResult<T>
    where F: Fn(HeapNode) -> Result<T, MachineError> {

    let (fn_addr, arg_addr) = try!(unwrap_heap_node_to_ap(m.heap.get(&ap_addr))); 
    let arg = m.heap.get(&arg_addr);
    
    //setup indirection
    if let HeapNode::Indirection(ind_addr) = arg {
        //rewrite the indirection node directly with the application node
        //application that does into the indirection address
        m.heap.rewrite(&ap_addr, 
                       HeapNode::Application {
                          fn_addr: fn_addr,
                          arg_addr: ind_addr
                      });
        return Result::Ok(HeapAccessValue::SetupExecution)
    };


    //it's not a data node, so this is something we need to still execute
    if !arg.is_data_node() {
        m.dump_stack(stack_to_dump);
        m.stack.push(arg_addr);
        return Result::Ok(HeapAccessValue::SetupExecution)
    }

    //give the node the access handler. it will either return the value
    //or fail to do so
    let access_result = try!(access_handler(arg));
    Result::Ok(HeapAccessValue::Found(access_result))
}

fn heap_try_num_access(h: HeapNode) -> Result<i32, MachineError> {
    match h {
        HeapNode::Num(i) => Result::Ok(i),
        other @ _ => Result::Err(format!(
                "expected number, found: {:#?}", other))
    }
}


fn heap_try_bool_access(h: HeapNode) -> Result<bool, MachineError> {
    match h {
        //TODO: make a separate function that takes HeapNode::Data
        //and returns the correct rust boolean
        HeapNode::Data{tag:0, ..} => Result::Ok(false),
        HeapNode::Data{tag: 1, ..} => Result::Ok(true),
        other @ _ => Result::Err(format!(
                "expected true / false, found: {:#?}", other))
    }
}


fn machine_is_final_state(m: &Machine) -> bool {
    assert!(m.stack.len() > 0, "expect stack to have at least 1 node");

    if m.stack.len() > 1 {
        false
    } else {
        let dump_empty = m.dump.len() == 0;
        m.heap.get(&m.stack.peek()).is_data_node() &&
        dump_empty
    }

}

//*** parsing & tokenisation ***

#[derive(Clone)]
enum ParseError {
    NoTokens,
    UnexpectedToken {
        expected: Vec<CoreToken>,
        found: CoreToken
    },
    ErrorStr(String),

}

impl fmt::Debug for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
           &ParseError::NoTokens => {
               write!(fmt, "no more tokens found")
           }
           &ParseError::UnexpectedToken{ref expected, ref found} => {
               write!(fmt, "expected one of {:#?}, \
                            found: |{:#?}|", expected, found)
           }
           &ParseError::ErrorStr(ref s) => write!(fmt, "{}",&s)
        }
    }
}


//*** tokenisation ***

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
enum CoreToken {
    Let,
    LetRec,
    In,
    Case,
    Ident(String),
    Assignment,
    Semicolon,
    OpenRoundBracket,
    CloseRoundBracket,
    OpenCurlyBracket,
    CloseCurlyBracket,
    Comma,
    Integer(String),
    Lambda,
    Or,
    And,
    L,
    LEQ,
    G,
    GEQ,
    EQ,
    NEQ,
    Plus,
    Minus,
    Mul,
    Div,
    Pack,
    //when you call peek(), it returns this token
    //if the token stream is empty.
    PeekNoToken
}

#[derive(Clone)]
struct ParserCursor {
    tokens: Vec<CoreToken>,
    pos: usize,
}

impl ParserCursor {
    fn new(tokens: Vec<CoreToken>) -> ParserCursor {
        ParserCursor {
            tokens: tokens,
            pos: 0
        }
    }

    fn peek(&self) -> CoreToken {
        match self.tokens.get(self.pos)
        .cloned() {
            Some(tok) => tok,
            None => CoreToken::PeekNoToken
        }

    }

    fn consume(&mut self) -> Result<CoreToken, ParseError> {
        match self.peek() {
            CoreToken::PeekNoToken => Result::Err(ParseError::NoTokens),
            other @ _ => {
                self.pos += 1;
                Result::Ok(other)
            }
        }

    }

    fn expect(&mut self, t: CoreToken) -> Result<(), ParseError> {
        let tok = self.peek();

        if tok == t {
            try!(self.consume());
            Result::Ok(())
        } else {
            Result::Err(ParseError::UnexpectedToken{
                expected: vec![t],
                found: tok
            })
        }
    }
}

fn identifier_str_to_token(token_str: &str) -> CoreToken {
    match token_str {
        "let" => CoreToken::Let,
        "letrec" => CoreToken::LetRec,
        "in" => CoreToken::In,
        "case" => CoreToken::Case,
        "Pack" => CoreToken::Pack,
        other @ _ => CoreToken::Ident(other.to_string())
    }
}


fn is_char_space(c: char) -> bool {
    c == ' ' || c == '\n' || c == '\t'
}

fn is_char_symbol(c: char) -> bool {
    !c.is_alphabetic() && !c.is_numeric()
}

fn tokenize_symbol(char_arr: Vec<char>, i: usize) -> 
    Result<(CoreToken, usize), ParseError> {
    

    let c = match char_arr.get(i) {
        Some(c) => c.clone(),
        None => return 
            Result::Err(ParseError::ErrorStr(format!(
                    "unable to get value out of: {} from: {:?}", i, char_arr)))
    };
    assert!(is_char_symbol(c),
    format!("{} is not charcter, digit or symbol", c));

    let symbol_token_map: HashMap<&str, CoreToken> =
        [("=", CoreToken::Assignment),
        (";", CoreToken::Semicolon),

        ("(", CoreToken::OpenRoundBracket),
        (")", CoreToken::CloseRoundBracket),

        ("{", CoreToken::OpenCurlyBracket),
        ("}", CoreToken::CloseCurlyBracket),

        (",", CoreToken::Comma),
        ("|", CoreToken::Or),
        ("&", CoreToken::And),
        ("<", CoreToken::L),
        ("<=", CoreToken::LEQ),
        (">", CoreToken::G),
        (">=", CoreToken::GEQ),

        ("!=", CoreToken::NEQ),
        ("==", CoreToken::EQ),
        //arithmetic
        ("+", CoreToken::Plus),
        ("-", CoreToken::Minus),
        ("*", CoreToken::Mul),
        ("/", CoreToken::Div),
        ("\\", CoreToken::Lambda)]
            .iter().cloned().collect();


    let longest_op_len = symbol_token_map
        .keys()
        .map(|s| s.len())
        .fold(0, cmp::max);


    //take only enough to not cause an out of bounds error
    let length_to_take = cmp::min(longest_op_len,
                                  char_arr.len() - i);

    //take all lengths, starting from longest,
    //ending at shortest
    let mut longest_op_opt : Option<CoreToken> = None;
    let mut longest_taken_length = 0;

    for l in (1..length_to_take+1).rev() {
        let op_str : &String = &char_arr[i..i + l]
            .iter()
            .cloned()
            .collect();

        if let Some(tok) = symbol_token_map.get(&op_str.as_str()) {
            //we found a token, break
            longest_taken_length = l;
            longest_op_opt = Some(tok.clone());
            break;
        }
    }

    //longest operator is tokenised
    let longest_op : CoreToken = match longest_op_opt {
        Some(op) => op,
        None => {
            let symbol = &char_arr[i..i + length_to_take];
            return Result::Err(ParseError::ErrorStr(format!(
                        "unknown symbol {:?}", symbol)))
        }
    };

    Result::Ok((longest_op, longest_taken_length))
    //tokens.push(longest_op);
    //i += longest_taken_length;
}



fn tokenize(program: String) -> Result<Vec<CoreToken>, ParseError> {

    //let char_arr : &[u8] = program.as_bytes();
    let char_arr : Vec<char> = program.clone().chars().collect();
    let mut i = 0;

    let mut tokens = Vec::new();

    loop {
        //break out if we have exhausted the loop
        if char_arr.get(i) == None {
            break;
        }

        //consume spaces
        while let Some(&c) = char_arr.get(i) {
            if !is_char_space(c) {
                break;
            }
            i += 1;
        }

        //we have a character
        if let Some(& c) = char_arr.get(i) {
            //alphabet: parse literal
            if c.is_alphabetic() {

                //get the identifier name
                let mut id_string = String::new();

                while let Some(&c) = char_arr.get(i) {
                    if c.is_alphanumeric() {
                        id_string.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }

                tokens.push(identifier_str_to_token(&id_string));
            }
            else if c.is_numeric() {
                //parse the number
                //TODO: take care of floats

                let mut num_string = String::new();

                while let Some(&c) = char_arr.get(i) {
                    if c.is_numeric() {
                        num_string.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }

                tokens.push(CoreToken::Integer(num_string));

            }
            else {
                let (symbol, stride) = try!(tokenize_symbol(char_arr.clone(), i));
                i += stride;
                tokens.push(symbol);
            }
        }

    }

    Result::Ok(tokens)

}

fn parse_string_as_int(num_str: String) -> Result<i32, ParseError> {
        i32::from_str_radix(&num_str, 10)
            .map_err(|_| ParseError::ErrorStr(format!(
                "unable to parse {} as int", num_str)))
}


//does this token allow us to start to parse an
//atomic expression?
fn is_token_atomic_expr_start(t: CoreToken) -> bool {
    match t {
        CoreToken::Integer(_) => true,
        CoreToken::Ident(_) => true,
        CoreToken::OpenRoundBracket => true,
        _ => false
    }

}


//atomic := <num> | <ident> | "(" <expr> ")"
fn parse_atomic_expr(mut c: &mut ParserCursor) ->
    Result<CoreExpr, ParseError> {
    match c.peek() {
        CoreToken::Integer(num_str) => {
            try!(c.consume());
            let num = try!(parse_string_as_int(num_str));

            Result::Ok(CoreExpr::Num(num))
        },
        CoreToken::Ident(ident) => {
            try!(c.consume());
            Result::Ok(CoreExpr::Variable(ident))
        },
        CoreToken::OpenRoundBracket => {
            try!(c.expect(CoreToken::OpenRoundBracket));
            let inner_expr = try!(parse_expr(&mut c));
            try!(c.expect(CoreToken::CloseRoundBracket));
            Result::Ok(inner_expr)
        },
        other @ _ =>
            return Result::Err(ParseError::ErrorStr(format!(
                "expected integer, identifier or (<expr>), found {:#?}",
                other)))
    }

}

//defn := <variable> "=" <expr>
fn parse_defn(mut c: &mut ParserCursor) ->
Result<(CoreVariable, Box<CoreExpr>), ParseError> {

    if let CoreToken::Ident(name) = c.peek() {
        try!(c.consume());
        try!(c.expect(CoreToken::Assignment));

        let rhs : CoreExpr = try!(parse_expr(&mut c));
        Result::Ok((name, Box::new(rhs)))

    }
    else {
        return Result::Err(ParseError::ErrorStr(format!(
                    "variable name expected at defn, found {:#?}", c.peek())));
    }
}

//let := "let" <bindings> "in" <expr>
fn parse_let(mut c: &mut ParserCursor) -> Result<CoreLet, ParseError> {
    //<let>
    let let_token = match c.peek() {
        CoreToken::Let => try!(c.consume()),
        CoreToken::LetRec => try!(c.consume()),
        _ => return Result::Err(ParseError::ErrorStr(format!(
                "expected let or letrec, found {:#?}", c.peek())))
    };

    let mut bindings : Vec<(Name, Box<CoreExpr>)> = Vec::new();

    //<bindings>
    loop {
        let defn = try!(parse_defn(&mut c));
        bindings.push(defn);

        //check for ;
        //If htere is a ;, continue parsing
        if let CoreToken::Semicolon = c.peek() {
            try!(c.consume());
            continue;
        }
        else {
            break;
        }
    }
    //<in>
    try!(c.expect(CoreToken::In));

    //<expr>
    let rhs_expr = try!(parse_expr(c));

    let is_rec : bool = match let_token {
        CoreToken::Let => false,
        CoreToken::LetRec => true,
        other @ _ =>
        return Result::Err(ParseError::UnexpectedToken {
            expected: vec![CoreToken::Let, CoreToken::LetRec],
            found: other.clone()
        })
    };

    Result::Ok(CoreLet {
        is_rec: is_rec,
        bindings: bindings,
        expr: Box::new(rhs_expr)
    })
}

//pack := Pack "{" tag "," arity "}"
fn parse_pack(mut c: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    try!(c.expect(CoreToken::Pack));
    try!(c.expect(CoreToken::OpenCurlyBracket));

    let tag : u32 = match c.peek() {
        CoreToken::Integer(s) => {
            try!(c.consume());
            try!(parse_string_as_int(s)) as u32
        }
        other @ _ => 
            return Result::Err(ParseError::ErrorStr(format!(
                    "expected integer tag, found {:#?}", other)))
    };

    try!(c.expect(CoreToken::Comma));

    let arity : u32 = match c.peek() {
        CoreToken::Integer(s) => {
            try!(c.consume());
            try!(parse_string_as_int(s)) as u32
        }
        other @ _ => 
            return Result::Err(ParseError::ErrorStr(format!(
                    "expected integer arity, found {:#?}", other)))
    };

    try!(c.expect(CoreToken::CloseCurlyBracket));
    Result::Ok(CoreExpr::Pack{tag: tag, arity: arity })


}


//aexpr := variable | number | Pack "{" num "," num "}" | "(" expr ")" 
fn parse_application(mut cursor: &mut ParserCursor) -> 
    Result<CoreExpr, ParseError> {
    let mut application_vec : Vec<CoreExpr> = Vec::new();
    loop {
        let c = cursor.peek();
        //we have a "pack" expression
        if let CoreToken::Pack = c {
            let pack_expr = try!(parse_pack(&mut cursor));
            application_vec.push(pack_expr);
        } else if is_token_atomic_expr_start(c) {
            let atomic_expr = try!(parse_atomic_expr(&mut cursor));
            application_vec.push(atomic_expr);
        } else {
            break;
        }
    }

    if application_vec.len() == 0 {
        Result::Err(ParseError::ErrorStr(
                concat!("wanted function application or atomic expr ",
                        "found neither").to_string()))

    }
    else if application_vec.len() == 1 {
        //just an atomic expr
        Result::Ok(application_vec.remove(0))
    }
    else {

        //function application
        //convert f g x  y to
        //((f g) x) y
        let mut cur_ap_lhs = {
            let ap_lhs = application_vec.remove(0);
            let ap_rhs = application_vec.remove(0);
            CoreExpr::Application(Box::new(ap_lhs), Box::new(ap_rhs))
        };

        //drop the first two and start folding
        for ap_rhs in application_vec.into_iter() {
            cur_ap_lhs = CoreExpr::Application(Box::new(cur_ap_lhs), Box::new(ap_rhs));
        }

        Result::Ok(cur_ap_lhs)
    }
}

fn parse_binop_at_precedence(mut cursor: &mut ParserCursor,
                             lhs_parse_fn: fn(&mut ParserCursor) -> Result<CoreExpr, ParseError>,
                             rhs_parse_fn: fn(&mut ParserCursor) -> Result<CoreExpr, ParseError>,
                             variable_bindings: HashMap<CoreToken, CoreExpr>) -> Result<CoreExpr, ParseError> {

    let lhs_expr : CoreExpr = try!(lhs_parse_fn(&mut cursor));

    let c : CoreToken = cursor.peek();

    let (rhs_expr, operator_variable) = {
        if let Some(&CoreExpr::Variable(ref op_str)) = variable_bindings.get(&c) {
            let op_var = CoreExpr::Variable(op_str.clone());
            try!(cursor.expect(c));
            let rhs = try!(rhs_parse_fn(&mut cursor));

            (rhs, op_var)
        }
        else {
            return Result::Ok(lhs_expr);
        }


    };

    let ap_inner =
    CoreExpr::Application(Box::new(operator_variable), Box::new(lhs_expr));

    Result::Ok(CoreExpr::Application(Box::new(ap_inner),
                                     Box::new(rhs_expr)))


}

fn parse_mul_div(mut cursor: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    parse_binop_at_precedence(cursor,
                              parse_application,
                              parse_mul_div,
                              [(CoreToken::Mul, CoreExpr::Variable("*".to_string())),
                              (CoreToken::Div, CoreExpr::Variable("/".to_string()))
                              ].iter().cloned().collect())
    //parse_application(&mut cursor)
}


fn parse_add_sub(mut cursor: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    parse_binop_at_precedence(cursor,
                              parse_mul_div,
                              parse_add_sub,
                              [(CoreToken::Plus, CoreExpr::Variable("+".to_string())),
                              (CoreToken::Minus, CoreExpr::Variable("-".to_string()))
                              ].iter().cloned().collect())
}

fn parse_relop(mut cursor: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    parse_binop_at_precedence(cursor,
                              parse_add_sub,
                              parse_relop,
                              [(CoreToken::L, CoreExpr::Variable("<".to_string())),
                              (CoreToken::LEQ, CoreExpr::Variable("<=".to_string())),
                              (CoreToken::G, CoreExpr::Variable(">".to_string())),
                              (CoreToken::GEQ, CoreExpr::Variable(">=".to_string())),
                              (CoreToken::EQ, CoreExpr::Variable("==".to_string())),
                              (CoreToken::NEQ, CoreExpr::Variable("!=".to_string()))
                              ].iter().cloned().collect())


}


//expr2 -> expr3 "&" expr2 | expr3
fn parse_and(mut cursor: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    parse_binop_at_precedence(cursor,
                              parse_relop,
                              parse_and,
                              [(CoreToken::And, CoreExpr::Variable("&".to_string()))
                              ].iter().cloned().collect())

}

//expr1 -> expr2 "|" expr1 | expr1
fn parse_or(mut cursor: &mut ParserCursor) -> Result<CoreExpr, ParseError> {
    parse_binop_at_precedence(cursor,
                              parse_and,
                              parse_or,
                              [(CoreToken::And, CoreExpr::Variable("|".to_string()))
                              ].iter().cloned().collect())
}




fn parse_expr(mut c: &mut ParserCursor) ->
Result<CoreExpr, ParseError> {
    match c.peek() {
        CoreToken::Let => parse_let(&mut c).map(|l| CoreExpr::Let(l)),
        CoreToken::LetRec => parse_let(&mut c).map(|l| CoreExpr::Let(l)),
        CoreToken::Case => panic!("cannot handle case yet"),
        CoreToken::Lambda => panic!("cannot handle lambda yet"),
        _ => parse_or(&mut c)
    }
}




fn string_to_program(string: String) -> Result<CoreProgram, ParseError> {

    let tokens : Vec<CoreToken> = try!(tokenize(string));
    let mut cursor: ParserCursor = ParserCursor::new(tokens);

    let mut program : CoreProgram = Vec::new();

    loop {
        if let CoreToken::Ident(sc_name) = cursor.peek() {
            try!(cursor.consume());

            let mut sc_args = Vec::new();
            //<args>* = <expr>
            while cursor.peek() != CoreToken::Assignment &&
            cursor.peek() != CoreToken::PeekNoToken {
                if let CoreToken::Ident(sc_arg) = cursor.peek() {
                    try!(cursor.consume());
                    sc_args.push(sc_arg);

                }
                else {
                    return Result::Err(ParseError::ErrorStr(format!(
                            "super combinator argument expected, \
                            {:#?} encountered",
                           cursor.consume())));
                }
            }
            //take the equals
            try!(cursor.expect(CoreToken::Assignment));
            let sc_body = try!(parse_expr(&mut cursor));

            program.push(SupercombDefn{
                name: sc_name,
                args: sc_args,
                body: sc_body
            });

            match cursor.peek() {
                //we ran out of tokens, this is the last SC
                //break
                CoreToken::PeekNoToken => break,
                //we got a ;, more SCs to come
                CoreToken::Semicolon => {
                    try!(cursor.expect(CoreToken::Semicolon));
                    continue
                },
                other @ _ => {
                    return Result::Err(ParseError::ErrorStr(format!(
                            "expected either ; or EOF, found {:#?}",
                                    other)));
                        }
            }

        } else {
            return Result::Err(ParseError::ErrorStr(format!(
                "super combinator name expected, {:#?} encountered",
                   cursor.consume())));
        }
    }
    Result::Ok(program)
}


#[cfg(test)]
fn run_machine(program:  &str) -> Machine {
    let main = string_to_program(program.to_string())
    .unwrap();
    let mut m = Machine::new(main);
    while !machine_is_final_state(&m) {
        let _ = m.step();
    }
    return m
}

#[test]
fn test_skk3() {
    let m = run_machine("main = S K K 3");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(3));
}

#[test]
fn test_negate_simple() {
    let m = run_machine("main = negate 1");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(-1));
}

#[test]
fn test_negate_inner_ap() {
    let m = run_machine("main = negate (negate 1)");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(1));
}


#[test]
fn test_add_simple() {
    let m = run_machine("main = 1 + 1");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(2));
}

#[test]
fn test_add_lhs_ap() {
    let m = run_machine("main = (negate 1) + 1");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(0));
}


#[test]
fn test_add_rhs_ap() {
    let m = run_machine("main = 1 + (negate 3)");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(-2));
}

#[test]
fn test_add_lhs_rhs_ap() {
    let m = run_machine("main = (negate 1) + (negate 3)");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(-4));
}

#[test]
fn test_complex_arith() {
    let m = run_machine("main = 1 * 2 + 10 * 20 + 30 / 3");
    assert!(m.heap.get(&m.stack.peek()) == HeapNode::Num(212));
}

// main ---
fn main() {
    use std::io::Write;
    let mut pause_per_step = true;

    loop {
        let mut input : String = String::new();
        print!("\n>>>");
        io::stdout().flush().unwrap();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {}
            Err(error) => panic!("error in read_line: {}", error)
        };

        //FIXME: why does this not work?    
        if input == "exit".to_string() {
            print!("input is EXIT");
            break;
        }
        else if &input == "step" {
            pause_per_step = true;
        }
        else if &input =="nostep" {
            pause_per_step = false;
        }
        else {
            let mut m : Machine = {
                let main = match string_to_program("main = ".to_string() + &input) {
                    Result::Ok(mut p) => p.remove(0),
                    Result::Err(e) => {
                        print!("error: {:#?}", e);
                        continue;
                    }
                };

                Machine::new(vec![main])
            };

            let mut i = 0;

            loop {
                match m.step() {
                    Result::Ok(env) => {
                        print!("*** ITERATION: {} \n***", i);
                        print_machine(&m, &env);
                    },
                    Result::Err(e) => {
                        print!("step error: {}\n", e);
                        break;
                    }
                };

                i += 1;
                
                if machine_is_final_state(&m) { break; }

                if pause_per_step {
                    let mut discard = String::new();
                    let _ = io::stdin().read_line(&mut discard);
                }
            }

            print!("*** MACHINE ENDED ***");

        }
    }
}
