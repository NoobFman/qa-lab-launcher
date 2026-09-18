import{defineConfig}from'vitest/config';
export default defineConfig({test:{include:['src/features/lab-alive/**/*.test.ts'],environment:'node'}});
