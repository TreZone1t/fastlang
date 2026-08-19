use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    /*
    first we need to identify where we ganna will or can to use  name
    1. as a normal variable (ptr variable)
        int(32) x = 10;
     -  name y = x;  // it will take the type name(int32) that means it is a smart pointer
       so we have a a type => name and a name => y and a reference => x
     - name<int32> y; y -> x;  // it will take the type name(int32) that means it is a smart pointer
       so we can give a type after the name
    - name y = new int(32)[10]; that will create a new array in the heap it is equivalent to
      - int(32)* y = new int(32)[10];
    - name y = new Node(); that will create a new object in the heap it is equivalent to
      - Node* y = new Node();
      we can pass a existing object to a name and it will we a reference to that object
      - Node y = new Node();
      - name z = y;  // it will be read only reference
      - name z = modify y;  // it will be a mutable reference
      - name z = copy y;  // it will be a copy of the object (mutable)
      - name z = new y;  // it will be new object with the type of the original object (Node) (mutable)
    2. as a parameter(ptr or ref) to unknown type
        fn my_add(x :name, y : int(32)) -> int(32) {
            try -> {
                int(32) result = &x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        we can pass in the name a expected type of course
        fn my_add(x : name<int(32)>, y : int(32)) -> int(32) {
            try -> {
                int(32) result = &x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        it can also be (mutable)
        fn my_add(x : modify name, y : int(32)) -> int(32) {
            try -> {
                x = int(x); // int(val) is not implemented yet
                int(32) result = &x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        it can also be (copy)
        fn my_add(x : copy name, y : int(32)) -> int(32) {
            try -> {
                int(32) result = &x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        and it can also be (new) //but it have no use for it
        fn my_add(x : new name, y : int(32)) -> int(32) {
            try -> {
                x = 10;
                int(32) result = &x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        that must important thing we can pass a name to a function and it will be a reference to the func
        fn my_add(x : name<fn>, y : int(32)) -> int(32) {
           try -> {
                int(32) result = &x() + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        also it can be a reference to a scope
        block my_scope -> {
            int(32) x = 10;
        }
        fn my_add(x : name<block>, y : int(32)) -> int(32) {
            try -> {
                int(32) result = &x.x + y;
                return result;
            } catch (e) ->{
                return 0;
            }
        }
        name it is like auto in c++  but it will be more safe

        so we need to have two functions
        1. parse_name
        2. parse_name_in_param : we will call it when we see name or modify name or copy name or new name in a param in any whare
     */

    // ====================================================================
    // `name` declarations — safe references (smart pointer)
    // ====================================================================
    //
    // Grammar:
    //   name_decl  ::= `name` [`<` type `>`] IDENT `=` expr `;`          -- ReadOnly
    //                | `name` [`<` type `>`] IDENT `->` expr `;`          -- ReadOnly / heap
    //                | `name` [`<` type `>`] IDENT `->` `modify` expr `;` -- ReadWrite
    //
    // Examples (from tests/19_magic_ref.fs):
    //   name safe_ptr = number;                   // ReadOnly ref to local
    //   name mut_ptr -> modify number;             // ReadWrite ref
    //   name not_safe_ptr -> new int(32)[...];     // heap-allocated, is_heap=true
    //   name<int(32)> bad_ptr -> safe_ptr;         // typed name, ReadOnly
    // ====================================================================
    pub(crate) fn parse_name(&mut self) -> Result<Decl, String> {
        // consume `name`
        self.advance();

        // Optional generic: `name<T>`
        let inner_type = if self.peek().kind == TokenKind::Less {
            self.advance(); // '<'
            let t = self.parse_type()?;
            self.consume(TokenKind::Greater, "Expected '>' after name type parameter")?;
            t
        } else {
            BaseType::Unknown
        };

        // variable name
        let var_name = self.get_identifier("Expected variable name after 'name'")?;

        // `=` or `->`
        let op = self.peek().kind.clone();
        if op != TokenKind::Assign && op != TokenKind::Arrow {
            return Err(
                format!(
                    "Syntax Error: Expected '=' or '->' after name declaration '{}', found '{}'",
                    var_name,
                    op.as_str()
                )
            );
        }
        self.advance(); // consume `=` or `->`

        // Check for `modify` keyword → ReadWrite
        let access_mode = if
            self.peek().kind == TokenKind::Modify ||
            self.peek().kind == TokenKind::New ||
            self.peek().kind == TokenKind::Copy
        {
            if self.peek().kind == TokenKind::Modify {
                self.advance(); // consume `modify`
            }
            AccessMode::ReadWrite
        } else {
            AccessMode::ReadOnly
        };

        // Parse the target expression
        let target = self.parse_expression()?;

        // Determine if heap-allocated (target came from `new`)
        let is_heap = matches!(&target, Expr::ArrayAllocate { .. } | Expr::Instantiate { .. });
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Decl::NameDecl {
            name: var_name,
            inner_type,
            target,
            access_mode,
            is_heap,
        })
    }
}
