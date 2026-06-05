/**
 * @omnilang/test-utils-vue — Test utilities for OmniView-generated Vue components.
 *
 * Wraps @vue/test-utils with helpers specific to OmniView's Vue 3 output:
 * - Pinia store mocking and injection
 * - data-testid query helpers
 * - v-model binding simulation
 * - Component fixture mounting with OmniView conventions
 *
 * Usage:
 *   import { mountOmni, findByTestId, fillInput } from '@omnilang/test-utils-vue';
 *
 *   const wrapper = mountOmni(MyComponent, {
 *     props: { title: 'Hello' },
 *     store: { useMyStore: { field: 'value' } },
 *   });
 *   await wrapper.findByTestId('btn-submit').trigger('click');
 */

const { mount, flushPromises } = require("@vue/test-utils");

/**
 * Mount an OmniView-generated Vue component with OmniView conventions.
 *
 * Injects Pinia stores, sets up global plugins, and provides
 * convenient wrappers for common test patterns.
 *
 * @param {Object} component - The Vue component (from .omni → .vue)
 * @param {Object} options - Mount options
 * @param {Object} options.props - Component props
 * @param {Object} options.store - Pinia store mocks: { storeName: { field: value } }
 * @param {Object} options.globals - Additional global options
 * @returns {Object} Enhanced wrapper with OmniView helper methods
 */
function mountOmni(component, options = {}) {
  const { props = {}, store = {}, globals = {}, ...mountOptions } = options;

  // Build global plugins from store mocks
  const plugins = [];

  // For each mocked store, create a Pinia instance with initial state
  if (Object.keys(store).length > 0) {
    try {
      const { createPinia, setActivePinia } = require("pinia");
      const pinia = createPinia();
      setActivePinia(pinia);

      // Set initial state for each mocked store
      for (const [storeName, state] of Object.entries(store)) {
        pinia.state.value[storeName] = state;
      }

      plugins.push(pinia);
    } catch (_) {
      // Pinia not installed — skip store mocking
    }
  }

  const wrapper = mount(component, {
    props,
    global: {
      plugins,
      ...globals,
    },
    ...mountOptions,
  });

  // ── OmniView helpers ──────────────────────────────────

  /**
   * Find element by data-testid attribute (OmniView's test-id prop).
   * @param {string} testId
   * @returns {Object} DOM wrapper
   */
  wrapper.findByTestId = (testId) => {
    return wrapper.find(`[data-testid="${testId}"]`);
  };

  /**
   * Find all elements matching a data-testid.
   * @param {string} testId
   * @returns {Array} Array of DOM wrappers
   */
  wrapper.findAllByTestId = (testId) => {
    return wrapper.findAll(`[data-testid="${testId}"]`);
  };

  /**
   * Get text content of an element found by test-id.
   * @param {string} testId
   * @returns {string}
   */
  wrapper.getByTestId = (testId) => {
    const el = wrapper.find(`[data-testid="${testId}"]`);
    return el.exists() ? el.text() : null;
  };

  /**
   * Check if an element with the given test-id exists.
   * @param {string} testId
   * @returns {boolean}
   */
  wrapper.hasTestId = (testId) => {
    return wrapper.find(`[data-testid="${testId}"]`).exists();
  };

  return wrapper;
}

/**
 * Simulate filling an input that uses v-model (OmniView bind:value).
 *
 * @param {Object} wrapper - The mounted component wrapper
 * @param {string} testId - data-testid of the input element
 * @param {string} value - Value to fill
 * @returns {Promise<void>}
 */
async function fillInput(wrapper, testId, value) {
  const input = wrapper.find(`[data-testid="${testId}"]`);
  if (!input.exists()) {
    throw new Error(`Input with test-id="${testId}" not found`);
  }
  await input.setValue(value);
}

/**
 * Simulate checking/unchecking a checkbox (OmniView bind:checked).
 *
 * @param {Object} wrapper - The mounted component wrapper
 * @param {string} testId - data-testid of the checkbox element
 * @param {boolean} checked - Whether to check or uncheck
 * @returns {Promise<void>}
 */
async function toggleCheckbox(wrapper, testId, checked = true) {
  const checkbox = wrapper.find(`[data-testid="${testId}"]`);
  if (!checkbox.exists()) {
    throw new Error(`Checkbox with test-id="${testId}" not found`);
  }
  await checkbox.setChecked(checked);
}

/**
 * Click a button found by test-id.
 *
 * @param {Object} wrapper - The mounted component wrapper
 * @param {string} testId - data-testid of the button
 * @returns {Promise<void>}
 */
async function clickButton(wrapper, testId) {
  const button = wrapper.find(`[data-testid="${testId}"]`);
  if (!button.exists()) {
    throw new Error(`Button with test-id="${testId}" not found`);
  }
  await button.trigger("click");
}

/**
 * Create a Pinia store mock for testing.
 *
 * @param {string} storeName - Name of the store (e.g., "useTodoStore")
 * @param {Object} initialState - Initial state values
 * @param {Object} actions - Mock action implementations
 * @returns {Object} Store mock suitable for mountOmni options.store
 */
function mockStore(storeName, initialState = {}, actions = {}) {
  const state = { ...initialState };

  // Build getter mocks
  const getters = {};
  for (const key of Object.keys(state)) {
    getters[key] = () => state[key];
  }

  return {
    [storeName]: {
      ...state,
      ...getters,
      ...actions,
    },
  };
}

module.exports = {
  mountOmni,
  fillInput,
  toggleCheckbox,
  clickButton,
  mockStore,
  flushPromises,
};
