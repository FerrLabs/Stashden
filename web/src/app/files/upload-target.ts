import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  input,
  output,
  viewChild,
} from '@angular/core';
import { fromFiles, type Outgoing } from '../outgoing';
import { PLATFORM } from '../platform';

@Component({
  selector: 'rx-upload-target',
  templateUrl: './upload-target.html',
  styleUrl: './upload-target.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UploadTarget {
  private readonly platform = inject(PLATFORM);

  readonly pending = input(0);
  readonly chosen = output<Outgoing[]>();

  private readonly input = viewChild.required<ElementRef<HTMLInputElement>>('picker');
  private readonly folder = viewChild.required<ElementRef<HTMLInputElement>>('folderPicker');

  protected open(): void {
    this.input().nativeElement.click();
  }

  protected openFolder(): void {
    this.folder().nativeElement.click();
  }

  protected pick(event: Event): void {
    const picker = event.target as HTMLInputElement;
    const files = Array.from(picker.files ?? []);
    picker.value = '';
    const upload = this.platform.upload;
    if (upload !== undefined && files.length > 0) {
      this.chosen.emit(fromFiles(upload, files));
    }
  }
}
