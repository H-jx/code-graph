"""Narrow, non-executing AST adapter for the investigated example ORM syntax."""
import ast
import pathlib

from tree_sitter import Language, Parser
import tree_sitter_typescript


def typescript_signature(node):
    if node.type == 'comment':
        return ''
    if not node.children:
        return node.text.decode('utf-8')
    return ''.join(typescript_signature(child) for child in node.children)


def decode_js_literal(raw):
    """Decode static JS literals; reject interpolation and unsupported escapes."""
    if len(raw) < 2 or raw[0] not in "'\"`" or raw[-1] != raw[0]:
        return None
    if raw[0] == '`' and '${' in raw:
        return None
    result, index = [], 1
    escapes = {'n': '\n', 'r': '\r', 't': '\t', 'b': '\b', 'f': '\f', 'v': '\v',
               '0': '\0', '\\': '\\', "'": "'", '"': '"', '`': '`', '$': '$'}
    try:
        while index < len(raw) - 1:
            char = raw[index]
            index += 1
            if char != '\\':
                result.append(char)
                continue
            char = raw[index]
            index += 1
            if char in escapes:
                if char == '0' and raw[index:index + 1].isdigit():
                    return None
                result.append(escapes[char])
            elif char in '\r\n':
                if char == '\r' and raw[index:index + 1] == '\n':
                    index += 1
            elif char in ('u', 'x'):
                if char == 'u' and raw[index] == '{':
                    end = raw.index('}', index)
                    result.append(chr(int(raw[index + 1:end], 16)))
                    index = end + 1
                else:
                    width = 4 if char == 'u' else 2
                    result.append(chr(int(raw[index:index + width], 16)))
                    index += width
            else:
                return None
        # A surrogate pair in an escaped JS literal is one Unicode scalar.
        return ''.join(result).encode('utf-16', 'surrogatepass').decode('utf-16')
    except (ValueError, IndexError, UnicodeError):
        return None


def parse_source(source):
    """Return bindings, explicit imports and top-level functions with byte spans."""
    content = source['content']
    data = content.encode('utf-8')
    bindings, imports, functions, exports = {}, {}, [], set()

    def bind(name, expression, span):
        bindings[name] = None if name in bindings or name in imports else (expression, span)

    if source['path'].endswith('.py'):
        tree = ast.parse(content)
        lines = data.splitlines(keepends=True)
        offsets = [0]
        for line in lines:
            offsets.append(offsets[-1] + len(line))

        def span(node):
            return (offsets[node.lineno - 1] + node.col_offset,
                    offsets[node.end_lineno - 1] + node.end_col_offset)

        def expression(node):
            if isinstance(node, ast.Name):
                return ('name', node.id)
            if isinstance(node, ast.Constant) and isinstance(node.value, str):
                return ('literal', node.value)
            if isinstance(node, (ast.List, ast.Tuple)):
                return ('array', [expression(item) for item in node.elts])
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                return ('call', expression(node.func.value), node.func.attr,
                        [expression(arg) for arg in node.args], span(node))
            return ('unknown',)

        for statement in tree.body:
            if isinstance(statement, ast.Assign):
                for target in statement.targets:
                    if isinstance(target, ast.Name):
                        bind(target.id, expression(statement.value), span(statement))
            elif isinstance(statement, ast.ImportFrom) and statement.level == 1 and statement.module:
                for alias in statement.names:
                    local = alias.asname or alias.name
                    imports[local] = None if local in imports or local in bindings else (statement.module.replace('.', '/') + '.py', alias.name)
            elif isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
                parameters = [arg.arg for arg in [*statement.args.posonlyargs, *statement.args.args, *statement.args.kwonlyargs]]
                if statement.args.vararg:
                    parameters.append(statement.args.vararg.arg)
                if statement.args.kwarg:
                    parameters.append(statement.args.kwarg.arg)
                blocked, calls = set(parameters), []

                def walk(node):
                    if node is not statement and isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                        blocked.add(node.name)
                        return
                    if isinstance(node, (ast.Lambda, ast.ListComp, ast.DictComp, ast.SetComp, ast.GeneratorExp)):
                        return
                    if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                        blocked.add(node.id)
                    if isinstance(node, ast.alias):
                        blocked.add(node.asname or node.name.split('.')[0])
                    if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                        calls.append(expression(node))
                    for child in ast.iter_child_nodes(node):
                        walk(child)
                walk(statement)
                functions.append(dict(name=statement.name, signature='(' + ','.join(parameters) + ')',
                                      span=span(statement), blocked=blocked, calls=calls))
    else:
        parser = Parser(Language(tree_sitter_typescript.language_typescript()))
        tree = parser.parse(data)
        if tree.root_node.has_error:
            raise ValueError('invalid TypeScript syntax')

        def text(node):
            return data[node.start_byte:node.end_byte].decode('utf-8')

        def span(node):
            return (node.start_byte, node.end_byte)

        def expression(node):
            if node is None:
                return ('unknown',)
            if node.type == 'identifier':
                return ('name', text(node))
            if node.type in ('string', 'template_string'):
                value = decode_js_literal(text(node))
                return ('literal', value) if value is not None else ('unknown',)
            if node.type == 'array':
                return ('array', [expression(child) for child in node.named_children])
            if node.type == 'call_expression':
                member = node.child_by_field_name('function')
                if member.type == 'member_expression':
                    return ('call', expression(member.child_by_field_name('object')),
                            text(member.child_by_field_name('property')),
                            [expression(child) for child in node.child_by_field_name('arguments').named_children], span(node))
            return ('unknown',)

        for node in tree.root_node.named_children:
            if node.type == 'export_statement':
                node = node.child_by_field_name('declaration')
                if node is None:
                    continue
                if node.type in ('lexical_declaration', 'variable_declaration'):
                    for declaration in node.named_children:
                        target = declaration.child_by_field_name('name')
                        if target is not None and target.type == 'identifier':
                            exports.add(text(target))
            if node.type in ('lexical_declaration', 'variable_declaration'):
                for declaration in node.named_children:
                    target = declaration.child_by_field_name('name')
                    if target is not None and target.type == 'identifier':
                        bind(text(target), expression(declaration.child_by_field_name('value')), span(declaration))
            elif node.type == 'import_statement':
                module = decode_js_literal(text(node.child_by_field_name('source')))
                def import_names(child):
                    if child.type == 'import_specifier':
                        target = child.child_by_field_name('name')
                        local = child.child_by_field_name('alias') or target
                        name = text(local)
                        imports[name] = None if name in imports or name in bindings else (module, text(target))
                    for part in child.named_children:
                        import_names(part)
                if module and module.startswith('.'):
                    import_names(node)
            elif node.type in ('function_declaration', 'generator_function_declaration'):
                parameters = node.child_by_field_name('parameters')
                blocked = {text(part) for child in parameters.named_children
                           for part in descendants(child.child_by_field_name('pattern') or child) if part.type == 'identifier'}
                calls = []
                def walk(child):
                    if child is not node and child.type in ('function_declaration', 'function_expression', 'arrow_function', 'class_declaration'):
                        name = child.child_by_field_name('name')
                        if name:
                            blocked.add(text(name))
                        return
                    if child.type in ('variable_declarator', 'assignment_expression'):
                        target = child.child_by_field_name('name') or child.child_by_field_name('left')
                        if target:
                            blocked.update(text(part) for part in descendants(target) if part.type == 'identifier')
                    if child.type == 'catch_clause':
                        target = child.child_by_field_name('parameter')
                        if target:
                            blocked.update(text(part) for part in descendants(target) if part.type == 'identifier')
                    if child.type == 'call_expression':
                        call = expression(child)
                        if call[0] == 'call':
                            calls.append(call)
                    for part in child.named_children:
                        walk(part)
                walk(node)
                functions.append(dict(name=text(node.child_by_field_name('name')), signature=typescript_signature(parameters),
                                      span=span(node), blocked=blocked, calls=calls))
    return dict(bindings=bindings, imports=imports, functions=functions, exports=exports)


def descendants(node):
    yield node
    for child in node.named_children:
        yield from descendants(child)


def imported_path(source_path, module):
    """Normalize only relative repository paths; resolution never leaves a repo."""
    parts = list(pathlib.PurePosixPath(source_path).parent.parts)
    for part in module.split('/'):
        if part == '..':
            if not parts:
                return None
            parts.pop()
        elif part and part != '.':
            parts.append(part)
    return '/'.join(parts)
