import * as Omni from './index'

declare global {
  interface Window {
    Omni: typeof Omni
  }
}

if (typeof window !== 'undefined') {
  window.Omni = Omni
}
