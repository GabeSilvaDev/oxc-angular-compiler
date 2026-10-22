/**
 * `(animate.enter)` / `(animate.leave)` listeners, in templates and host bindings.
 */
import type { Fixture } from '../types.js'

export const fixtures: Fixture[] = [
  {
    name: 'animate-enter-listener',
    category: 'animations',
    description: '(animate.enter) listener on an element',
    className: 'AnimateEnterListenerComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-animate-enter-listener',
  template: \`<div (animate.enter)="onEnter($event)">Content</div>\`,
})
export class AnimateEnterListenerComponent {
  onEnter(event: unknown) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵanimateEnterListener'],
  },
  {
    name: 'animate-leave-listener-in-if',
    category: 'animations',
    description: '(animate.leave) listener next to a static animate.leave inside @if',
    className: 'AnimateLeaveListenerInIfComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-animate-leave-listener-in-if',
  template: \`
    @if (visible) {
      <div animate.leave="fade-out" (animate.leave)="left = true; onLeave($event)">Content</div>
    }
  \`,
})
export class AnimateLeaveListenerInIfComponent {
  visible = true;
  left = false;
  onLeave(event: unknown) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵanimateLeave', 'ɵɵanimateLeaveListener'],
  },
  {
    name: 'animate-host-listener-and-attribute',
    category: 'animations',
    description: 'host (animate.leave) listener and static animate.enter attribute',
    className: 'AnimateHostComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-animate-host',
  template: '<ng-content />',
  host: { '(animate.leave)': 'onLeave($event)', 'animate.enter': 'fade-in' },
})
export class AnimateHostComponent {
  onLeave(event: unknown) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵanimateLeaveListener', 'ɵɵanimateEnter'],
  },
  {
    name: 'animate-host-listener-decorator',
    category: 'animations',
    description: '@HostListener on animate.enter in a directive',
    className: 'AnimateHostListenerDirective',
    type: 'full-transform',
    sourceCode: `
import { Directive, HostListener } from '@angular/core';

@Directive({ selector: '[appAnimateHost]' })
export class AnimateHostListenerDirective {
  @HostListener('animate.enter', ['$event'])
  onEnter(event: unknown) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵanimateEnterListener'],
  },
  {
    name: 'animate-listener-canonical-on-prefix',
    category: 'animations',
    description: 'on-animate.enter, the canonical form of (animate.enter)',
    className: 'AnimateOnPrefixComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-animate-on-prefix',
  template: \`<div on-animate.enter="onEnter($event)">Content</div>\`,
})
export class AnimateOnPrefixComponent {
  onEnter(event: unknown) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵanimateEnterListener'],
  },
  {
    name: 'animate-listener-on-ng-template',
    category: 'animations',
    description: '(animate.leave) on an ng-template stays a plain listener',
    className: 'AnimateNgTemplateComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-animate-ng-template',
  template: \`<ng-template (animate.leave)="onLeave($event)" (click)="onClick()"></ng-template>\`,
})
export class AnimateNgTemplateComponent {
  onLeave(event: unknown) {}
  onClick() {}
}
    `.trim(),
    expectedFeatures: ['ɵɵdomListener'],
  },
]
