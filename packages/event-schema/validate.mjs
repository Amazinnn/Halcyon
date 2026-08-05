import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const schema = JSON.parse(
  readFileSync(fileURLToPath(new URL("./agent-event.schema.json", import.meta.url)), "utf8"),
);

const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const validate = ajv.compile(schema);

const list = (dir) =>
  readdirSync(fileURLToPath(new URL(`./fixtures/${dir}/`, import.meta.url)))
    .filter((f) => f.endsWith(".json"))
    .sort();

const load = (dir, file) =>
  JSON.parse(readFileSync(fileURLToPath(new URL(`./fixtures/${dir}/${file}`, import.meta.url)), "utf8"));

let failed = 0;
const validFiles = list("valid");
const invalidFiles = list("invalid");

for (const file of validFiles) {
  if (!validate(load("valid", file))) {
    failed += 1;
    console.error(`VALID fixture FAILED (should pass): ${file}`);
    console.error(JSON.stringify(validate.errors, null, 2));
  }
}

for (const file of invalidFiles) {
  if (validate(load("invalid", file))) {
    failed += 1;
    console.error(`INVALID fixture PASSED (should fail): ${file}`);
  }
}

if (failed > 0) {
  console.error(`event-schema validation: ${failed} failure(s)`);
  process.exit(1);
}
console.log(
  `event-schema validation OK: ${validFiles.length} valid, ${invalidFiles.length} invalid fixtures checked.`,
);