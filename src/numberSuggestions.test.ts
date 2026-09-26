import { describe,expect,it } from "vitest";
import { suggestRecordNumber } from "./numberSuggestions";

describe("record number suggestions",()=>{
 it("starts with a conservative canonical project number",()=>{expect(suggestRecordNumber([],"RFI")).toBe("RFI-001");expect(suggestRecordNumber([],"SUB")).toBe("SUB-001")});
 it("advances canonical numbers case-insensitively and preserves established padding",()=>{expect(suggestRecordNumber(["rfi-001","RFI-009"],"RFI")).toBe("RFI-010");expect(suggestRecordNumber(["SUB-0007"],"SUB")).toBe("SUB-0008")});
 it("does not infer unrelated customer conventions",()=>{expect(suggestRecordNumber(["A-12","SK-004","RFI-002"],"RFI")).toBe("RFI-003")});
});
