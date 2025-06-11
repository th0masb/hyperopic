import * as path from "path";
import {CargoBinNames, LambdaType} from "./cargo";
import { Stack } from "aws-cdk-lib";
import { aws_lambda as lambda } from "aws-cdk-lib";
import { aws_iam as iam } from "aws-cdk-lib";
import { Construct } from "constructs";
import {AccountAndRegion, LambdaConfig, OpeningTableConfig} from "../config";
import {Platform} from "aws-cdk-lib/aws-ecr-assets";

export class BotStack extends Stack {
  readonly moveLambdaName: string;
  private readonly id: string;

  constructor(
      scope: Construct,
      id: string,
      accountAndRegion: AccountAndRegion,
      lambdaConfig: LambdaConfig,
      openingTableConfig: OpeningTableConfig,
  ) {
    super(scope, id, {env: accountAndRegion});
    this.id = id;
    this.moveLambdaName = this.functionName(LambdaType.Move)
    for (const type of [LambdaType.Move, LambdaType.Benchmark]) {
      const cargoBinName = CargoBinNames.get(type)!
      const name = this.functionName(type);
      const fn = new lambda.DockerImageFunction(this, name, {
        functionName: name,
        retryAttempts: 0,
        memorySize: lambdaConfig.memoryMB,
        timeout: lambdaConfig.timeout,
        architecture: lambda.Architecture.ARM_64,
        code: lambda.DockerImageCode.fromImageAsset(
          path.join(__dirname, "..", "..", ".."),
          {
            platform: Platform.LINUX_ARM64,
            file: path.join("tools", "workspace.dockerfile"),
            buildArgs: {
              APP_NAME: cargoBinName,
              APP_CONFIG: JSON.stringify({
                name: openingTableConfig.tableName,
                region: accountAndRegion.region,
                positionKey: openingTableConfig.positionAttributeName,
                moveKey: openingTableConfig.movesAttributeName,
                maxDepth: openingTableConfig.maxDepth,
              })
            },
          }
        ),
      });
      const ps = new iam.PolicyStatement();
      ps.addActions("dynamodb:GetItem");
      const {region, account} = accountAndRegion;
      ps.addResources(`arn:aws:dynamodb:${region}:${account}:table/${openingTableConfig.tableName}`);
      fn.addToRolePolicy(ps);
    }
  }

  private functionName(type: LambdaType): string {
    return `${this.id}-${LambdaType[type]}`;
  }
}

