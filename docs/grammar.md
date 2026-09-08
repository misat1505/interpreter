## Grammar

### Syntax Part

**program** = { import_declaration | struct_declaration | enum_declaration | function_declaration | extern_function_declaration | assign_or_call | if_statement | for_statement | while_statement | switch_statement | (declaration, ";") | let_declaration };

**comment** = "#" , {unicode_character - "\n"}, "\n";

**import_declaration** = "import", literal, ";";
```
import "../utils.rp";
```

**struct_declaration** = "struct", identifier, "{", [ struct_members ], "}", ";";
```text
struct Hobby {
  u64 id,
  str description
};

struct Person {
  u64 id,
  str name,
};
```

**struct_members** = struct_member, { ",", struct_member };

**struct_member** = type, identifier;

**enum_declaration** = "enum", identifier, "{", [ enum_members ], "}", ";";
```text
enum Task {
    InProgress(Deadline),
    Completed(str),
    Aborted
};
```

**enum_members** = enum_member, { ",", enum_member };

**enum_member** = identifier, [ "(", type, ")" ];

**extern_function_declaration** = "extern", "fn", identifier, "(", parameters, ")", ":", type | "void", [ "as", identifier ] ";";

```text
extern fn InitWindow(i64 x, i64 y, str name): void as init_window;
extern fn PrintValue(i64 value): void as print_value;
```

**function_declaration** = "fn", identifier, "(", parameters, ")", ":", type | "void", statement_block;

```text
fn is_prime(i64 x, &i64 total_iters): bool {
    return true;
}

fn add(i64 a, i64 b): i64 {
    return a + b;
}
```

**parameters** = [ parameter, { ",", parameter } ];

**parameter** = ["&"], type, identifier;

**statement_block** = ("{", {statement}, "}") | statement;

**statement** = assign_or_call | if_statement | for_statement | while_statement | switch_statement | (declaration, ";") | let_declaration | return_statement | break_statement | continue_statement;

**assign_or_call_without_semicolon** = identifier, ( { access_tail }, ("=" | "+=" | "-=" | "*=" | "/=" | "%="), expression | "(", arguments, ")");

**assign_or_call** = assign_or_call_without_semicolon, ";";

```text
x = 5;
my_fun(5, 2);

x[0][0] = 10;
x[1][0] += 2;

result = add(10, 20);
```

**declaration** = type, identifier, [ "=", expression ];

```text
bool is_valid = true;
i64 counter = 0;
f64 result = 10.5;
str message = "Hello, Raptor!";
```

**let_declaration** = let, identifier, [ ":", identifier ], "=", expression, ";";

```text
let is_valid = true;
let counter = 0;
let result = 10.5;
let message = "Hello, Raptor!";
let colors = ["white", "black"];
let array: str[] = [];
```

**if_statement** = "if", "(", expression, ")", statement_block, [ "else", statement_block ];

```text
if (x == 5) {
    println("x is five.");
} else {
    println("x is not five.");
}
```

**for_statement** = "for", "(", [ declaration ], ";", expression, ";", [ assign_or_call_without_semicolon ], ")", statement_block;

```text
for (i64 i = 0; i < 10; i = i + 1) {
    println(i as str);
}
```

**while_statement** = "while", "(", expression, ")", statement_block

```text
while (x < 5) {
    x += 1;
}
```

A `for` loop may also omit its initialization and increment expressions:

```text
i64 i = 0;

for (; i < 10 ;) {
    i = i + 1;
}
```

**break_statement** = "break", ";";

```text
break;
```

**continue_statement** = "continue", ";";

```text
continue;
```

**return_statement** = "return", [ expression ], ";";

```text
return a + 2 * b;
```

**argument** = ["&"], expression;

**arguments** = [ argument, {",", argument} ];

```text
a + 2
&b
c
```

Multiple arguments are separated by commas:

```text
calculate(a + 2, &b, c);
```

**expression** = concatenation_term { "||", concatenation_term };

```text
a == b && b || c
```

**concatenation_term** = relation_term, { "&&", relation_term };

```text
a == b && b
```

**relation_term** = additive_term, [ relation_operands, additive_term ];

```text
x == y
x >= y
```

**additive_term** = multiplicative_term , { ("+" | "-"), multiplicative_term };

```text
1 + (1 + 2) / (2 + 3)
x + 10 - y
```

**multiplicative_term** = casted_term, { ("*" | "/" | "%"), casted_term };

```text
(1 + 2) / (2 + 3)
x * 10 % 3
```

**casted_term** = unary_term, [ "as", type ];

```text
(x + add(2, 2)) as f64

2 as i64      # 2
2 as f64      # 2.0
2 as str      # "2"
-2 as str     # "-2"
2 as bool     # true
0 as bool     # false

"123" as i64  # 123
"fdsfs" as i64 # error

"" as bool    # false
"a" as bool   # true
```

**unary_term** = [ ("-", "!") ], factor;

```text
-2
-(x + 5)

!true
!(x == 5)
```

**factor** = literal | ( "(", expression, ")" ) | identifier_or_call_or_struct_literal | vector_literal;

```text
5
2.2
(2.2 + 3 as f64)

x
fun(5)
```

**vector_literal** = "[", [ expression, { ",", expression } ], "]";

```text
[]

[1, 2, 3]

["hello", "world"]

[
    [1, 2],
    [3, 4]
]
```

**identifier_or_call_or_struct_literal** = identifier, ( call_or_index_tail | struct_literal_tail );
```text
x
fun(5)

x[0][0]
fun(5)[0][0]

Person { id: 123 as u64, name: "Bob" }
Hobby { id: 1, description: "Play football" }

person.hobbies[0].description
```

<!-- **call_or_index_tail** = [ "(", arguments, ")" ], { "[", expression, "]" }; -->
**call_or_index_tail** = [ "(", arguments, ")" ], { access_tail };
**access_tail** = ("[", expression, "]") | (".", identifier);

**struct_literal_tail** = "{", [ struct_literal_fields ], "}";

**struct_literal_fields** = struct_literal_field, { ",", struct_literal_field };

**struct_literal_field** = identifier, ":", expression;


**literal** = integer_literal | float_literal | boolean_literal | string_literal | char_literal;

**identifier** = letter, {character};

```text
super_variable_123
counter
result_value
```

**switch_statement** = "switch", "(", switch_expressions, ")", "{", {switch_case}, "}";

**switch_expression** = expression, [ ":", identifier ];

**switch_expressions** = switch_expression, { ",", switch_expression };

**switch_case** = "(", expression, ")", "->", statement_block;

```text
switch (x: temp1, y: temp2) {
    (x < 5 && temp2 < 5) -> {
        print("Less than 5.");
    }

    (temp1 < 10 && y < 10) -> {
        print("Less than 10.");
        break;
    }
}
```

### Lexical Part

**letter** = "a" - "z" | "A" - "Z";

**type** = ("i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f64" | "bool" | "char" | "str"), { "[", "]" };

**relation_operands** = "==" | "<" | "<=" | ">" | ">=" | "!=";

**digit** = "0" - "9";

**non_zero_digit** = "1" - "9";

**integer_literal** = ( non_zero_digit, {digit} ) | "0";

```text
1
12
10
0
```

**float_literal** = integer_literal, ".", {digit}

```text
1.0
1.2
10.0
0.0
0.00001
```

**string_literal** = "\", {unicode_character - "\"}, "\";

**boolean_literal** = "true" | "false";

**character** = "a" - "z" | "A" - "Z" | "0" - "9" | "_";

**unicode_character** = (all unicode characters)

## Operator priority

| Operator              | Priority |
| --------------------- | -------: |
| `-` (number negation) |        7 |
| `!`                   |        7 |
| `as`                  |        6 |
| `*`                   |        5 |
| `/`                   |        5 |
| `%`                   |        5 |
| `+`                   |        4 |
| `-` (subtraction)     |        4 |
| `>`                   |        3 |
| `>=`                  |        3 |
| `<`                   |        3 |
| `<=`                  |        3 |
| `==`                  |        3 |
| `!=`                  |        3 |
| `&&`                  |        2 |
| `\|\|`                |        1 |
