import { describe, expect, it } from 'vitest'

import { extractComponentMetadataSync, transformAngularFileSync } from '../index.js'

// The queries reported must be the ones the component is compiled with.
describe('extractComponentMetadataSync queries', () => {
  it('resolves const selectors and includes `queries:` metadata after member queries', () => {
    const [component] = extractComponentMetadataSync(
      `
import { Component, ContentChild, ViewChild } from '@angular/core';

const SEL = 'ref';

@Component({
  selector: 'app-x',
  template: '<div #ref></div>',
  queries: {
    fromMetaView: new ViewChild('metaRef'),
    fromMetaContent: new ContentChild('metaContent', { descendants: false }),
  },
})
export class X {
  @ViewChild(SEL) member: any;
  @ContentChild('memberContent') memberContent: any;
}
`,
      'x.component.ts',
    )

    expect(component.viewQueries?.map((q) => [q.propertyName, q.predicate])).toEqual([
      ['member', '["ref"]'],
      ['fromMetaView', '["metaRef"]'],
    ])
    expect(component.queries?.map((q) => [q.propertyName, q.predicate, q.descendants])).toEqual([
      ['memberContent', '["memberContent"]', true],
      ['fromMetaContent', '["metaContent"]', false],
    ])
  })
})

// ngtsc reads `@NS.Input()` and the other member decorators only when `NS` is a
// namespace import of `@angular/core`.
describe('extractComponentMetadataSync namespaced member decorators', () => {
  const source = (module: string) => `
import { Component } from '@angular/core';
import * as NS from '${module}';

@Component({ selector: 'app-x', template: '<div #ref></div>' })
export class X {
  @NS.Input() value!: string;
  @NS.Output() changed: any;
  @NS.ViewChild('ref') ref: any;
  @NS.HostListener('click') onClick() {}
}
`

  it('reads them through a namespace import of @angular/core', () => {
    const [component] = extractComponentMetadataSync(source('@angular/core'), 'x.component.ts')
    expect(component.inputs?.map((i) => i.classPropertyName)).toEqual(['value'])
    expect(component.outputs?.map((o) => o.classPropertyName)).toEqual(['changed'])
    expect(component.viewQueries?.map((q) => q.propertyName)).toEqual(['ref'])
    expect(component.host?.listeners).toEqual([['(click)', 'onClick()']])
  })

  it('ignores them through any other namespace', () => {
    const [component] = extractComponentMetadataSync(source('foreign-decorators'), 'x.component.ts')
    expect(component.inputs ?? []).toEqual([])
    expect(component.outputs ?? []).toEqual([])
    expect(component.viewQueries ?? []).toEqual([])
    expect(component.host?.listeners ?? []).toEqual([])
  })
})

// Like ngtsc, a signal member counts only when it calls Angular's own `input()`,
// `output()`, `model()`, `viewChild()`, ...: imported from `@angular/core` by
// name (under any alias) or through a namespace import.
describe('extractComponentMetadataSync signal members', () => {
  it('reads only the ones that call Angular APIs', () => {
    const [component] = extractComponentMetadataSync(
      `
import { Component, input as inp, output } from '@angular/core';
import * as ng from '@angular/core';
import { model, viewChild } from './other';
function contentChild(x: string): any { return null; }

@Component({ selector: 'app-x', template: '<div #ref></div>' })
export class X {
  a = inp(0);
  b = ng.model.required<string>();
  c = output();
  d = model(0);
  e = viewChild('ref');
  f = contentChild('ref');
  g = ng.viewChild('ref');
}
`,
      'x.component.ts',
    )
    expect(component.inputs?.map((i) => [i.classPropertyName, i.isSignal])).toEqual([
      ['a', true],
      ['b', true],
    ])
    expect(component.outputs?.map((o) => o.classPropertyName)).toEqual(['b', 'c'])
    expect(component.viewQueries?.map((q) => q.propertyName)).toEqual(['g'])
    expect(component.queries ?? []).toEqual([])
  })
})

// Like ngtsc, an `@Input(...)` / `@Output(...)` argument is evaluated, so a
// same-file const gives the alias, `required` flag and transform.
describe('extractComponentMetadataSync member decorator arguments', () => {
  it('reads aliases and options through same-file consts', () => {
    const [component] = extractComponentMetadataSync(
      `
import { Component, EventEmitter, Input, Output, booleanAttribute } from '@angular/core';
const NAME = 'y';
const OPTS = { alias: \`z\`, required: true, transform: booleanAttribute };
const t = booleanAttribute;

@Component({ selector: 'app-x', template: '' })
export class X {
  @Input(NAME) a: any;
  @Input(OPTS) b: any;
  @Input({ ...OPTS, alias: 'w', transform: t }) c: any;
  @Output('e' + NAME) d = new EventEmitter();
}
`,
      'x.component.ts',
    )
    expect(
      component.inputs?.map((i) => [
        i.classPropertyName,
        i.bindingPropertyName,
        i.required,
        i.transform ?? null,
      ]),
    ).toEqual([
      ['a', 'y', false, null],
      ['b', 'z', true, 'booleanAttribute'],
      ['c', 'w', true, 'booleanAttribute'],
    ])
    expect(component.outputs?.map((o) => [o.classPropertyName, o.bindingPropertyName])).toEqual([
      ['d', 'ey'],
    ])
  })
})

// Like the compiler (and ngtsc), only Angular's `@Component` counts: imported
// from `@angular/core` under any name or through a namespace import.
describe('extractComponentMetadataSync class decorators', () => {
  it("lists the classes Angular's @Component decorates, not another library's", () => {
    const components = extractComponentMetadataSync(
      `
import { Component as Cmp } from '@angular/core';
import * as ng from '@angular/core';
import { Component } from './widgets';

@Cmp({ selector: 'app-a', template: '' })
export class A {}

@ng.Component({ selector: 'app-b', template: '' })
export class B {}

@Component({ selector: 'x-widget', template: '' })
export class Widget {}
`,
      'x.component.ts',
    )
    expect(components.map((c) => c.className)).toEqual(['A', 'B'])
  })
})

// The styles reported must be the ones the component is compiled with.
describe('extractComponentMetadataSync styles', () => {
  const source = (styles: string) => `
import { Component } from '@angular/core';
import { OTHER } from './other';

const S = '.a{}';
const LIST = ['.l{}'];

@Component({ selector: 'app-x', template: '', styles: ${styles} })
export class X {}
`

  it.each([
    ['[S]', ['.a{}']],
    ['S', ['.a{}']],
    ["['.b{}' + '.c{}']", ['.b{}.c{}']],
    ['[...LIST]', ['.l{}']],
    ['LIST', ['.l{}']],
    ['[]', []],
  ])('evaluates `styles: %s` as the transform does', (styles, expected) => {
    const [component] = extractComponentMetadataSync(source(styles), 'x.component.ts')
    expect(component.styles).toEqual(expected)

    const result = transformAngularFileSync(source(styles), 'x.component.ts')
    expect(result.errors).toEqual([])
    for (const style of expected) {
      expect(result.code).toContain(style.replaceAll('{}', '[_ngcontent-%COMP%]{}'))
    }
  })

  it.each(['[OTHER]', '[S, OTHER]', '[S, 1]'])(
    'reports `styles: %s` instead of dropping an entry',
    (styles) => {
      const code = source(styles)
      const [error] = transformAngularFileSync(code, 'x.component.ts').errors
      expect(error.message).toMatch(
        /^(@Component\.styles depends on 'OTHER'|Failed to resolve styles at position 1 to a string)/,
      )
      expect(error.labels.map((label) => code.slice(label.start, label.end))).toEqual([styles])
      expect(extractComponentMetadataSync(code, 'x.component.ts')[0].styles).toEqual([])
    },
  )
})

// `host` is evaluated the way the compiler evaluates it, so a spread or a
// constant reports the same host as the entries written out.
describe('extractComponentMetadataSync host', () => {
  const ENTRIES = `'(click)': 'go()', '[title]': 't', '[class.x]': 'on', '[style.width]': 'w', '[attr.aria-label]': 't', role: 'button', class: 'a b', style: 'color: red'`

  function hostOf(preamble: string, host: string) {
    const [component] = extractComponentMetadataSync(
      `import { Component } from '@angular/core';
${preamble}
@Component({ selector: 'app-x', template: '', host: ${host} })
export class X { go() {} t = ''; on = true; w = ''; }
`,
      'x.component.ts',
    )
    return component.host
  }

  const literal = hostOf('', `{ ${ENTRIES} }`)

  it('reports every kind of entry for a literal host', () => {
    expect(literal).toEqual({
      properties: [
        ['[title]', 't'],
        ['[class.x]', 'on'],
        ['[style.width]', 'w'],
        ['[attr.aria-label]', 't'],
      ],
      attributes: [['role', 'button']],
      listeners: [['(click)', 'go()']],
      classAttr: 'a b',
      styleAttr: 'color: red',
    })
  })

  it('reports a dynamic attribute value as the expression it is emitted as', () => {
    expect(
      hostOf(`declare const foo: any`, `{ role: foo.bar(), 'aria-label': 'L' }`)?.attributes,
    ).toEqual([
      ['role', 'foo.bar()'],
      ['aria-label', 'L'],
    ])
  })

  it('reports the same host for a spread', () => {
    expect(hostOf(`const ALL = { ${ENTRIES} };`, '{ ...ALL }')).toEqual(literal)
    expect(
      hostOf(
        `const A = { '(click)': 'go()', '[title]': 't', '[class.x]': 'on', '[style.width]': 'w' };
const B = { ...A, '[attr.aria-label]': 't', role: 'link' };`,
        `{ ...B, role: 'button', class: 'a b', style: 'color: red' }`,
      ),
    ).toEqual(literal)
  })

  it('reports the same host for a constant', () => {
    expect(hostOf(`const HOST = { ${ENTRIES} };`, 'HOST')).toEqual(literal)
    expect(hostOf(`const HOST = { ${ENTRIES} } as const;`, 'HOST')).toEqual(literal)
  })
})
