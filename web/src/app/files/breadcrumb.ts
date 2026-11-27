import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { linkTo, segmentsOf } from '../folder';
import { t } from '../i18n/language';

@Component({
  selector: 'rx-breadcrumb',
  imports: [RouterLink],
  templateUrl: './breadcrumb.html',
  styleUrl: './breadcrumb.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Breadcrumb {
  protected readonly t = t;
  readonly path = input.required<string>();

  protected readonly segments = computed(() => segmentsOf(this.path()));
  protected readonly link = linkTo;
}
