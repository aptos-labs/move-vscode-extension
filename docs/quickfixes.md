## Code suggestions (with fixes)

### Use method call notation

<img src="images/replace_with_method_call.gif" alt="replace_with_method_call" width="650"/>

### Use compound assignment expression

<img src="images/compound_expr.gif" alt="compound_expr" width="650"/>

### Use vector index expr

Detects expressions of form `*vector::borrow(&some_vector, index)` and `*some_vector.borrow(index)`, 
which can be converted to `some_vector[index]`. 

<img src="images/vector_index_expr.gif" alt="vector_index_expr" width="650"/>

### Use field initialization shorthand

Detects struct literal fields which could be written in shorthand form.

<img src="images/field_shorthand.gif" alt="field_shorthand" width="650"/>

### Redundant integer type cast

Detects expressions like `number as u8`, where `number` is already of type it's being casted to.

<img src="images/redundant_cast.gif" alt="redundant_cast" width="650"/>









