import { createUsersFromSigners, getChainMetadata } from "./helpers";

async function main() {
  await getChainMetadata();
  await createUsersFromSigners();
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
