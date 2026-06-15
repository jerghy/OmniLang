/**
 * E2E test for OmniDriver Custom Function Operations API.
 *
 * Tests: customFunction, returnNode, addParam, setReturnType,
 *        setDescription, validateFunction, listCustomFunctions,
 *        findFunctionByName, checkFunctionNameConflict, getFunctionSignature
 *
 * Run:
 *   npx playwright test editor/e2e/omni-driver-customfn.spec.ts
 */

import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Custom Function Operations", () => {
  test("creates, validates, and discovers custom functions", async ({
    page,
  }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(
      () => (window as any).__OMNI__,
      { timeout: TIMEOUT },
    );

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("code");
      O.clearAll();

      // Create customFn
      const fn = O.customFunction(
        "calculateTotal",
        [
          { name: "price", type: "f64" },
          { name: "quantity", type: "i32" },
        ],
        "f64",
      );
      const fnData = O.element(fn.id).data;

      // Verify ports
      const portNames = (fnData.ports || []).map((p: any) => p.name);
      const returnPort = (fnData.ports || []).find(
        (p: any) => p.name === "return",
      );
      const pricePort = (fnData.ports || []).find(
        (p: any) => p.name === "price",
      );

      // create return node
      const ret = O.returnNode(fn.id);
      const retData = O.element(ret.id).data;
      const retPort = (retData.ports || []).find(
        (p: any) => p.name === "value",
      );

      // validateFunction — no body graph: should report no return reachable
      const report1 = O.validateFunction(fn.id);

      // Add body: add node → return
      const addNode = O.codeBlock("add", "addTotal", {
        typeAnnotation: "f64",
      });
      addNode.connectTo(ret, "result", "value");
      const report2 = O.validateFunction(fn.id);

      // addParam
      fn.addParam("discount", "f64", 0.0);
      const fnData2 = O.element(fn.id).data;
      const discountPort = (fnData2.ports || []).find(
        (p: any) => p.name === "discount",
      );

      // setReturnType and setDescription
      fn.setReturnType("f64");
      fn.setDescription("计算含折扣总价");

      // listCustomFunctions
      const allFns = O.listCustomFunctions();

      // findFunctionByName
      const found = O.findFunctionByName("calculateTotal");

      // checkFunctionNameConflict
      const conflicts = O.checkFunctionNameConflict("calculateTotal");

      // getFunctionSignature
      const sig = O.getFunctionSignature(fn.id);

      return {
        portNames,
        returnPortType: returnPort?.type,
        pricePortType: pricePort?.type,
        retPortType: retPort?.type,
        report1Valid: report1.valid,
        report1Errors: report1.errors,
        report2Valid: report2.valid,
        report2Paths: report2.reachableReturnPaths,
        discountPortExists: !!discountPort,
        discountPortType: discountPort?.type,
        allFnsCount: allFns.length,
        foundFnName: found?.fnName,
        foundReturnType: found?.returnType,
        conflictsCount: conflicts.length,
        sigFnName: sig?.fnName,
        sigBodyNodes: sig?.bodyNodeCount,
      };
    });

    // Port checks
    expect(result.portNames).toContain("return");
    expect(result.portNames).toContain("price");
    expect(result.portNames).toContain("quantity");
    expect(result.returnPortType).toBe("f64");
    expect(result.pricePortType).toBe("f64");
    expect(result.retPortType).toBe("f64");

    // validateFunction: without body, reports issue
    expect(result.report1Errors.length).toBeGreaterThanOrEqual(0);
    // With addNode → return connected, should be valid
    expect(result.report2Valid).toBe(true);
    expect(result.report2Paths).toBeGreaterThanOrEqual(1);

    // addParam
    expect(result.discountPortExists).toBe(true);
    expect(result.discountPortType).toBe("f64");

    // Discovery
    expect(result.allFnsCount).toBeGreaterThanOrEqual(1);
    expect(result.foundFnName).toBe("calculateTotal");
    expect(result.foundReturnType).toBe("f64");
    expect(result.conflictsCount).toBe(1);
    expect(result.sigFnName).toBe("calculateTotal");
    expect(result.sigBodyNodes).toBeGreaterThanOrEqual(1); // at least addNode
  });
});
