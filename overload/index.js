const express = require("express");
const app = express();
const port = 4000;

const { PrismaClient } = require("@prisma/client");

const prisma = new PrismaClient({
  // log: [
  //   {
  //     emit: "stdout",
  //     level: "query",
  //   },
  //   {
  //     emit: "stdout",
  //     level: "error",
  //   },
  //   {
  //     emit: "stdout",
  //     level: "info",
  //   },
  //   {
  //     emit: "stdout",
  //     level: "warn",
  //   },
  // ],
});

// let prisma = new PrismaClient({
//   log: [
//     {
//       emit: "event",
//       level: "query",
//     },
//   ],
// });

prisma.$on("query", async (e) => {
  console.log(`${e.query} ${e.params}`);
});

app.get("/", async (req, res) => {
  try {
    let users = await prisma.user.findMany();

    let promises = users.map(({ id }) =>
      prisma.post.count({ where: { userId: id } })
    );

    let results = await Promise.all(promises);
    res.status(200).json({ count: results });
  } catch (e) {
    res.status(500).json(e);
  }
});

app.listen(port, () => {
  console.log(`Example app listening on port ${port}`);
});
