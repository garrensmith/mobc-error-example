-- CreateTable
CREATE TABLE "post" (
    "id" VARCHAR(36) NOT NULL,
    "description" VARCHAR(255),
    "userId" VARCHAR(36) NOT NULL,

    CONSTRAINT "post_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "user" (
    "id" VARCHAR(36) NOT NULL,
    "name" VARCHAR(255) NOT NULL,

    CONSTRAINT "user_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "FK_user_id___post_userId" ON "post"("userId");

-- AddForeignKey
ALTER TABLE "post" ADD CONSTRAINT "FK_user_id_post_userId" FOREIGN KEY ("userId") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE CASCADE;
