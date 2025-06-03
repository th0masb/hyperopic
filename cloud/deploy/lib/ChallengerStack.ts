import {
    aws_lambda as lambda,
    Duration,
    Stack,
} from "aws-cdk-lib";
import {Construct} from "constructs";
import {AccountAndRegion, BotChallengerConfig} from "../config";
import * as path from "path";
import {Schedule, ScheduleExpression, ScheduleTargetInput} from "aws-cdk-lib/aws-scheduler";
import {LambdaInvoke} from "aws-cdk-lib/aws-scheduler-targets";

export class ChallengerStack extends Stack {
    constructor(
        scope: Construct,
        id: string,
        accountAndRegion: AccountAndRegion,
        config: BotChallengerConfig,
    ) {
        super(scope, id, {env: accountAndRegion});
        const challenge_function = new lambda.DockerImageFunction(this, id, {
            functionName: id,
            retryAttempts: 0,
            memorySize: 128,
            timeout: Duration.minutes(3),
            code: lambda.DockerImageCode.fromImageAsset(
                path.join(__dirname, "..", "..", ".."),
                {
                    file: path.join("tools", "workspace.dockerfile"),
                    buildArgs: {
                        APP_NAME: "challenge",
                        APP_CONFIG: JSON.stringify({
                            token: config.token,
                            ourUserId: config.ourUserId
                        })
                    },
                }
            ),
        });

        if (config.challengeSchedule) {
            const schedule = config.challengeSchedule
            new Schedule(this, `${id}-Schedule`, {
                description: "Challenge other bots",
                schedule: ScheduleExpression.expression(schedule.schedule),
                target: new LambdaInvoke(challenge_function, {
                    input: ScheduleTargetInput.fromObject(schedule.payload)
                }),
            })
        }
    }
}
