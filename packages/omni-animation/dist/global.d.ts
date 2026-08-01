import * as Omni from './index';
declare global {
    interface Window {
        Omni: typeof Omni;
    }
}
