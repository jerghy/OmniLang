/**
 * OmniView Vite Plugin — compiles .omni files to React/Vue components
 * with Hot Module Replacement (HMR) support.
 *
 * Usage (vite.config.js):
 *   import omniView from '@omnilang/vite-plugin-omniview';
 *
 *   export default {
 *     plugins: [
 *       omniView({ target: 'react' }),
 *     ],
 *   };
 *
 * The plugin:
 * 1. Intercepts .omni file imports
 * 2. Compiles them via `omnic build` to the configured target
 * 3. Generates virtual modules or writes to .omni-cache/
 * 4. Watches .omni files and triggers HMR on changes
 */

const path = require("path");
const fs = require("fs");
const { execSync, spawn } = require("child_process");

const PLUGIN_NAME = "vite-plugin-omniview";
const VIRTUAL_PREFIX = "\0omni:";

/**
 * Find the omnic CLI executable.
 * Checks local venv, global Python, and project-local virtual environments.
 */
function findOmnic(cwd) {
  // Try the project's virtual environment first
  const candidates = [
    path.join(cwd, ".venv", "Scripts", "omnic"),
    path.join(cwd, ".venv", "Scripts", "omnic.exe"),
    path.join(cwd, "venv", "Scripts", "omnic"),
    path.join(cwd, "node_modules", ".bin", "omnic"),
    "omnic", // Fall back to PATH
    "python", // Python direct (with -m omniview.cli)
  ];

  for (const candidate of candidates) {
    try {
      if (candidate === "omnic" || candidate === "python") {
        return candidate;
      }
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    } catch (_) {
      // Continue
    }
  }
  return "omnic"; // Default fallback
}

/**
 * Compile a .omni file and return the generated files.
 */
function compileOmniFile(omniPath, target, outputDir) {
  // Remove outputDir before building to get fresh files
  if (fs.existsSync(outputDir)) {
    fs.rmSync(outputDir, { recursive: true, force: true });
  }
  fs.mkdirSync(outputDir, { recursive: true });

  try {
    execSync(`omnic build "${omniPath}" --target ${target} --output "${outputDir}"`, {
      encoding: "utf-8",
      stdio: "pipe",
      timeout: 30000,
    });
  } catch (err) {
    // Try Python direct as fallback
    try {
      const pythonCmd = process.platform === "win32" ? "python" : "python3";
      execSync(
        `${pythonCmd} -m omniview.cli build "${omniPath}" --target ${target} --output "${outputDir}"`,
        {
          encoding: "utf-8",
          stdio: "pipe",
          timeout: 30000,
          cwd: path.resolve(__dirname, "..", "..", ".."),
        }
      );
    } catch (err2) {
      console.error(`[${PLUGIN_NAME}] Compilation failed for ${omniPath}:`);
      console.error(err2.stderr || err2.message);
      throw err2;
    }
  }

  // Find generated files
  const files = {};
  try {
    const entries = fs.readdirSync(outputDir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isFile()) {
        const filePath = path.join(outputDir, entry.name);
        files[entry.name] = fs.readFileSync(filePath, "utf-8");
      }
    }
  } catch (err) {
    console.error(`[${PLUGIN_NAME}] Failed to read generated files:`, err.message);
  }

  return files;
}

/**
 * Find the main component module from generated files.
 * Returns the .tsx (React) or .vue (Vue) file, excluding .css and store files.
 */
function findMainModule(files, omniName) {
  const keys = Object.keys(files);
  // Prefer the component file (matches the .omni filename)
  const componentFile =
    keys.find((k) => k.endsWith(".tsx") || k.endsWith(".vue")) ||
    keys[0];
  return { filename: componentFile, content: files[componentFile] };
}

/**
 * Resolve .omni imports to their compiled output.
 */
function omniResolvePlugin(target) {
  return {
    name: `${PLUGIN_NAME}:resolve`,

    resolveId(source, importer) {
      if (source.endsWith(".omni")) {
        const resolved = path.resolve(
          importer ? path.dirname(importer) : process.cwd(),
          source
        );
        return VIRTUAL_PREFIX + resolved;
      }
      return null;
    },

    load(id) {
      if (id.startsWith(VIRTUAL_PREFIX)) {
        const omniPath = id.slice(VIRTUAL_PREFIX.length);

        // Verify the .omni file exists
        if (!fs.existsSync(omniPath)) {
          console.error(`[${PLUGIN_NAME}] File not found: ${omniPath}`);
          return `export default function() { return null; }`;
        }

        const omniName = path.basename(omniPath, ".omni");
        const cacheDir = path.join(
          path.dirname(omniPath),
          ".omni-cache",
          omniName
        );

        // Compile the .omni file
        let files;
        try {
          files = compileOmniFile(omniPath, target, cacheDir);
        } catch (err) {
          this.error(`Failed to compile ${omniPath}: ${err.message}`);
          return "";
        }

        // Generate a virtual module that re-exports everything
        if (Object.keys(files).length === 0) {
          return `export default function() { return null; }`;
        }

        // For React: convert .tsx to virtual module with CSS import
        // For Vue: re-export the .vue SFC
        const tsxFiles = Object.keys(files).filter((k) => k.endsWith(".tsx"));
        const vueFiles = Object.keys(files).filter((k) => k.endsWith(".vue"));
        const cssFiles = Object.keys(files).filter((k) => k.endsWith(".css"));
        const storeFiles = Object.keys(files).filter(
          (k) => k.includes("Store") || k.includes("store")
        );

        let code = "";

        if (target === "react" || target === "dioxus") {
          // Import CSS modules
          for (const cssFile of cssFiles) {
            const cssPath = path.join(cacheDir, cssFile).replace(/\\/g, "/");
            code += `import "${cssPath}";\n`;
          }
          // Re-export store
          for (const storeFile of storeFiles) {
            code += `export * from "${path.join(cacheDir, storeFile).replace(/\\/g, "/")}";\n`;
          }
          // Re-export component
          for (const tsxFile of tsxFiles) {
            code += `export * from "${path.join(cacheDir, tsxFile).replace(/\\/g, "/")}";\n`;
          }
        } else if (target === "vue") {
          for (const vueFile of vueFiles) {
            code += `export { default } from "${path.join(cacheDir, vueFile).replace(/\\/g, "/")}";\n`;
          }
        }

        if (!code) {
          // Fallback: export all generated files
          for (const [filename] of Object.entries(files)) {
            const filePath = path.join(cacheDir, filename).replace(/\\/g, "/");
            if (filename.endsWith(".css")) {
              code += `import "${filePath}";\n`;
            } else {
              code += `export * from "${filePath}";\n`;
            }
          }
        }

        return code;
      }
      return null;
    },
  };
}

/**
 * HMR plugin: watches .omni source files and triggers hot reload.
 */
function omniHmrPlugin(target) {
  return {
    name: `${PLUGIN_NAME}:hmr`,

    configureServer(server) {
      // Watch all .omni files in the project
      server.watcher.add("**/*.omni");

      server.watcher.on("change", (filePath) => {
        if (!filePath.endsWith(".omni")) return;

        console.log(`[${PLUGIN_NAME}] ${filePath} changed, recompiling...`);

        const omniName = path.basename(filePath, ".omni");
        const cacheDir = path.join(
          path.dirname(filePath),
          ".omni-cache",
          omniName
        );

        try {
          compileOmniFile(filePath, target, cacheDir);
        } catch (err) {
          console.error(`[${PLUGIN_NAME}] HMR compile failed:`, err.message);
          return;
        }

        // Trigger HMR for all modules that import this .omni file
        const virtualId = VIRTUAL_PREFIX + path.resolve(filePath);
        const mod = server.moduleGraph.getModuleById(virtualId);
        if (mod) {
          server.moduleGraph.invalidateModule(mod);
          server.ws.send({ type: "full-reload" });
        }
      });
    },
  };
}

/**
 * Main OmniView Vite plugin.
 *
 * @param {Object} options
 * @param {string} options.target - Target framework: 'react' (default), 'vue', 'dioxus'
 * @param {string} options.omnicPath - Path to omnic executable (auto-detected if not specified)
 * @returns {Array} Array of Vite plugin objects
 */
function omniView(options = {}) {
  const { target = "react" } = options;

  return [omniResolvePlugin(target), omniHmrPlugin(target)];
}

module.exports = omniView;
module.exports.omniResolvePlugin = omniResolvePlugin;
module.exports.omniHmrPlugin = omniHmrPlugin;
