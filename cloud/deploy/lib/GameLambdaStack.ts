import {CargoBinNames, LambdaType} from "./cargo";
import * as path from "path";
import {aws_iam as iam, aws_lambda as lambda, Stack} from "aws-cdk-lib";
import {Construct} from "constructs";
import {AccountAndRegion, LambdaConfig} from "../config";
import {Platform} from "aws-cdk-lib/aws-ecr-assets";

export class GameLambdaStack extends Stack {

    readonly functionArn: string

    constructor(
        scope: Construct,
        id: string,
        accountAndRegion: AccountAndRegion,
        lambdaConfig: LambdaConfig,
        functionName: string,
        moveFunctionName: string
    ) {
        super(scope, id, {env: accountAndRegion});
        const cargoBinName = CargoBinNames.get(LambdaType.LichessGame)!
        const fn = new lambda.DockerImageFunction(this, id, {
            functionName: functionName,
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
                        APP_CONFIG: ""
                    },
                }
            ),
        });
        const ps = new iam.PolicyStatement();
        ps.addActions("lambda:InvokeFunction");
        const {region, account} = accountAndRegion;
        ps.addResources(
            `arn:aws:lambda:${region}:${account}:function:${moveFunctionName}`,
            `arn:aws:lambda:${region}:${account}:function:${functionName}`,
        )
        fn.addToRolePolicy(ps);
        this.functionArn = fn.functionArn
    }
}
