/**
 * Regression: Template locals inside $safeNavigationMigration.
 *
 * Issue: Template locals read inside $safeNavigationMigration(...) in
 * listeners and arrow functions must resolve to the restored-view
 * variables instead of the component context (#585).
 */
import type { Fixture } from '../types.js'

export const fixtures: Fixture[] = [
  {
    name: 'safe-nav-migration-in-listener',
    category: 'regressions',
    description: '$safeNavigationMigration in listeners reading template locals',
    className: 'SafeNavMigrationInListenerComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-safe-nav-migration-in-listener',
  standalone: true,
  template: \`
      @for (option of options; track $index) {
        <button (click)="pick($safeNavigationMigration(option?.value))">{{ option?.label }}</button>
        <button (click)="pick($safeNavigationMigration(selected?.value))">Selected</button>
      }
      @if (user; as u) {
        <button (click)="save($event, $safeNavigationMigration(u?.flags?.verified))">Save</button>
      }
      <ng-template let-item>
        <button (click)="pick($safeNavigationMigration(item?.value))">Item</button>
      </ng-template>
      <input #box />
      <button (click)="pick($safeNavigationMigration(box?.value))">Box</button>
      @let firstOption = options[0];
      <button (click)="pick($safeNavigationMigration(firstOption?.value))">First</button>
    \`,
})
export class SafeNavMigrationInListenerComponent {
  options: Array<{ value: string; label: string } | null> = [];
  selected: { value: string } | null = null;
  user: { flags?: { verified: boolean } } | null = null;
  pick(value: string | null) {}
  save(event: Event, verified: boolean | null) {}
}
    `.trim(),
    expectedFeatures: ['ɵɵlistener', 'ɵɵreference', 'ɵɵreadContextLet'],
  },
  {
    name: 'safe-nav-migration-in-arrow-function',
    category: 'regressions',
    description: '$safeNavigationMigration in arrow functions reading template locals',
    className: 'SafeNavMigrationInArrowFunctionComponent',
    type: 'full-transform',
    sourceCode: `
import { Component } from '@angular/core';

@Component({
  selector: 'app-safe-nav-migration-in-arrow-function',
  standalone: true,
  template: \`
      @for (option of options; track $index) {
        <div [title]="fmt(() => $safeNavigationMigration(option?.value))">
          {{ fmt(() => $safeNavigationMigration(option?.label)) }}
        </div>
      }
    \`,
})
export class SafeNavMigrationInArrowFunctionComponent {
  options: Array<{ value: string; label: string } | null> = [];
  fmt(fn: () => string | null) {
    return fn() ?? '';
  }
}
    `.trim(),
    expectedFeatures: ['ɵɵarrowFunction', 'ɵɵrestoreView', 'ɵɵresetView'],
  },
]
